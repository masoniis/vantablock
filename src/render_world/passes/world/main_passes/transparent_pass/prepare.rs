use crate::render_world::passes::world::gpu_resources::world_uniforms::{
    ChunkStorageBindGroupLayout, ChunkStorageManager,
};
use crate::{
    prelude::*,
    render_world::{
        global_extract::resources::RenderMeshStorageResource,
        graphics_context::resources::{RenderDevice, RenderQueue},
        passes::world::main_passes::{
            opaque_pass::extract::RenderTransformComponent,
            transparent_pass::extract::TransparentRenderMeshComponent,
        },
        types::upload_voxel_mesh,
    },
    simulation_world::asset_management::{AssetStorageResource, MeshAsset},
};
use bevy::ecs::prelude::*;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use tracing::warn;

#[instrument(skip_all)]
pub fn prepare_transparent_meshes_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    chunk_storage_layout: Res<ChunkStorageBindGroupLayout>,
    cpu_mesh_assets: Res<AssetStorageResource<MeshAsset>>,
    mut chunk_memory_manager: ResMut<ChunkStorageManager>,
    meshes_to_prepare: Query<(&TransparentRenderMeshComponent, &RenderTransformComponent)>,

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
                        "Prepared transparent GPU mesh for handle ID {}",
                        handle_id
                    );

                    entry.insert(Arc::new(gpu_mesh));
                }
            } else {
                warn!(
                    "Mesh asset for handle ID {} not found in AssetStorage (transparent pass).",
                    handle_id
                );
            }
        }
    }
}
