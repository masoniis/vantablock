use crate::render_world::passes::world::gpu_resources::world_uniforms::{
    ChunkStorageBindGroupLayout, ChunkStorageManager,
};
use crate::{
    ecs_core::SimToRenderReceiver,
    prelude::*,
    render_world::{
        global_extract::resources::RenderMeshStorageResource,
        graphics_context::resources::{RenderDevice, RenderQueue},
        passes::world::main_passes::opaque_pass::extract::{
            OpaqueRenderMeshComponent, RenderTransformComponent,
        },
        types::upload_voxel_mesh,
    },
    simulation_world::asset_management::{AssetStorageResource, MeshAsset},
};
use bevy::ecs::prelude::*;
use std::collections::hash_map::Entry;

/// A system to read all render meshes and initialize GPU buffers if they don't have one yet.
#[instrument(skip_all)]
pub fn prepare_opaque_meshes_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    chunk_storage_layout: Res<ChunkStorageBindGroupLayout>,
    cpu_mesh_assets: Res<AssetStorageResource<MeshAsset>>,
    mut chunk_memory_manager: ResMut<ChunkStorageManager>,
    meshes_to_prepare: Query<(&OpaqueRenderMeshComponent, &RenderTransformComponent)>,

    // Output (storage insertion)
    mut gpu_mesh_storage: ResMut<RenderMeshStorageResource>,
) {
    for (render_mesh, transform) in meshes_to_prepare.iter() {
        let handle = render_mesh.mesh_handle;
        let handle_id = handle.id();

        // if the GPU mesh for this handle doesn't exist yet, create it.
        if let Entry::Vacant(entry) = gpu_mesh_storage.meshes.entry(handle_id) {
            // get the asset data
            if let Some(mesh_asset) = cpu_mesh_assets.get(handle) {
                // create the GPU buffer
                let world_pos = transform.transform.w_axis.truncate().to_array();
                if let Some(gpu_mesh) = upload_voxel_mesh(
                    &mut chunk_memory_manager,
                    &device,
                    &queue,
                    &chunk_storage_layout,
                    &mesh_asset.faces,
                    world_pos,
                ) {
                    debug!(
                        target : "gpu_mesh_prepared",
                        "Prepared opaque GPU mesh for handle ID {}",
                        handle_id
                    );

                    entry.insert(Arc::new(gpu_mesh));
                }
            } else {
                warn!(
                    "Mesh asset for handle ID {} not found in AssetStorage (opaque pass).",
                    handle_id
                );
            }
        }
    }
}

/// A system that reads the cross-world command queue and deletes the corresponding
/// GPU buffer objects from the render mesh storage.
#[instrument(skip_all)]
pub fn delete_gpu_buffers_system(
    receiver_res: Res<SimToRenderReceiver>,
    mut chunk_memory_manager: ResMut<ChunkStorageManager>,
    mut gpu_mesh_storage: ResMut<RenderMeshStorageResource>,
) {
    for command in receiver_res.0.try_iter() {
        let handle_id = command.mesh_handle.id();
        if let Some(mesh) = gpu_mesh_storage.meshes.remove(&handle_id) {
            // free the allocation
            chunk_memory_manager.free_chunk(*mesh);

            debug!(
                target: "gpu_mesh_cleanup",
                "Successfully removed and implicitly dropped GPU mesh for handle ID {}.",
                handle_id
            );
        } else {
            warn!(
                "Attempted to clean up GPU mesh {} that was not found in storage.",
                handle_id
            );
        }
    }
}
