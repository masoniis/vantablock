use crate::render::pipeline::gpu_resources::world_uniforms::{
    ChunkStorageBindGroupLayout, ChunkStorageManager,
};
use crate::{
    prelude::*,
    render::data::{MeshesToUploadQueue, RenderMeshStorageResource},
};
use bevy::ecs::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use std::sync::Arc;

/// A system that processes the `MeshesToUploadQueue` and uploads new or modified
/// voxel meshes to the global GPU storage buffer.
#[instrument(skip_all)]
pub fn prepare_transparent_meshes_system(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layout: Res<ChunkStorageBindGroupLayout>,
    mut chunk_memory_manager: ResMut<ChunkStorageManager>,
    mut gpu_mesh_storage: ResMut<RenderMeshStorageResource>,
    mut meshes_to_upload: ResMut<MeshesToUploadQueue>,
) {
    // 1. Handle removals
    for id in meshes_to_upload.removals.drain(..) {
        if let Some(mesh_handle) = gpu_mesh_storage.meshes.remove(&id) {
            chunk_memory_manager.free_chunk(*mesh_handle);
            debug!(target: "gpu_mesh_cleanup", "Freed GPU allocation for mesh {:?}", id);
        }
    }

    // 2. Handle uploads (Added/Modified)
    for (id, mesh, world_pos) in meshes_to_upload.queue.drain(..) {
        // If it already exists, free the old allocation first
        if let Some(old_mesh_handle) = gpu_mesh_storage.meshes.remove(&id) {
            chunk_memory_manager.free_chunk(*old_mesh_handle);
        }

        // Allocate and upload to global SSBO
        if let Some(voxel_mesh) = chunk_memory_manager.allocate_chunk(
            &device,
            &queue,
            &layout,
            &mesh.faces,
            world_pos.into(),
        ) {
            gpu_mesh_storage.meshes.insert(id, Arc::new(voxel_mesh));
            debug!(
                target: "gpu_mesh_upload",
                "Uploaded mesh {:?} to global SSBO at pos {:?}", id, world_pos
            );
        }
    }
}
