use crate::prelude::*;
use crate::render::global_extract::resources::MeshesToUploadQueue;
use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::prelude::{MessageReader, Query, Res, ResMut};
use bevy::render::Extract;
use shared::simulation::asset_management::mesh_asset::VoxelChunkMeshAsset;
use shared::simulation::chunk::{
    OpaqueMeshComponent, TransformComponent, TransparentMeshComponent,
};

/// A system that extracts `VoxelChunkMeshAsset` events from the simulation world
/// and queues them for GPU upload in the render world.
#[instrument(skip_all)]
pub fn extract_modified_chunk_meshes(
    mut events: Extract<MessageReader<AssetEvent<VoxelChunkMeshAsset>>>,
    assets: Extract<Res<Assets<VoxelChunkMeshAsset>>>,
    opaque_meshes: Extract<Query<(&OpaqueMeshComponent, &TransformComponent)>>,
    transparent_meshes: Extract<Query<(&TransparentMeshComponent, &TransformComponent)>>,
    mut upload_queue: ResMut<MeshesToUploadQueue>,
) {
    // 1. Build a lookup of AssetId -> WorldPos for the current frame's extraction
    // Since AssetEvent only gives us the ID, we need to find which entity (and thus what position)
    // this asset belongs to. In this engine, it's typically 1:1 for chunks.
    let mut pos_lookup = std::collections::HashMap::new();
    for (mesh, transform) in opaque_meshes.iter() {
        pos_lookup.insert(mesh.mesh_handle.id(), transform.position);
    }
    for (mesh, transform) in transparent_meshes.iter() {
        pos_lookup.insert(mesh.mesh_handle.id(), transform.position);
    }

    // 2. Process events
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let mesh_id = *id;
                if let Some(asset) = assets.get(mesh_id) {
                    let world_pos = pos_lookup.get(&mesh_id).copied().unwrap_or(Vec3::ZERO);
                    // Clone is cheap because faces is an Arc
                    upload_queue.queue.push((mesh_id, asset.clone(), world_pos));
                }
            }
            AssetEvent::Removed { id } => {
                upload_queue.removals.push(*id);
            }
            _ => {}
        }
    }
}
