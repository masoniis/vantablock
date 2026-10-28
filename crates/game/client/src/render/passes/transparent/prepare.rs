use crate::render::chunk::gpu_storage::{ChunkGpuStorage, ChunkStorageBindGroupLayout};
use crate::{
    prelude::*,
    render::chunk::{MeshesToUploadQueue, RenderMeshStorageResource},
};
use bevy::ecs::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use std::sync::Arc;

/// A system that processes the `MeshesToUploadQueue` and uploads new or modified
/// block meshes to the global GPU storage buffer.
#[instrument(skip_all)]
pub fn prepare_transparent_meshes_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layout: Res<ChunkStorageBindGroupLayout>,
    mut gpu_storage: ResMut<ChunkGpuStorage>,
    mut gpu_mesh_storage: ResMut<RenderMeshStorageResource>,
    mut meshes_to_upload: ResMut<MeshesToUploadQueue>,
) {
    // handle removals
    for id in meshes_to_upload.removals.drain(..) {
        if let Some(mesh_handle) = gpu_mesh_storage.meshes.remove(&id) {
            gpu_storage.free_chunk(*mesh_handle);
            debug!(target: "gpu_mesh_cleanup", "Freed GPU allocation for mesh {:?}", id);
        }
    }

    // handle uploads (Added/Modified)
    for (id, mesh, world_pos) in meshes_to_upload.additions.drain(..) {
        // if it already exists, free the old allocation first
        if let Some(old_mesh_handle) = gpu_mesh_storage.meshes.remove(&id) {
            gpu_storage.free_chunk(*old_mesh_handle);
        }

        // allocate and upload to global SSBO
        if let Some(block_mesh) =
            gpu_storage.allocate_chunk(&device, &queue, &layout, &mesh.faces, world_pos.into())
        {
            gpu_mesh_storage.meshes.insert(id, Arc::new(block_mesh));
            debug!(
                target: "gpu_mesh_upload",
                "Uploaded mesh {:?} to global SSBO at pos {:?}", id, world_pos
            );
        }
    }
}
