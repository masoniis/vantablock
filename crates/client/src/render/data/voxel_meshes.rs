use crate::prelude::*;
use crate::render::data::MeshesToUploadQueue;
use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::prelude::{Added, Changed, MessageReader, Or, Query, Res, ResMut};
use bevy::render::Extract;
use shared::simulation::asset::VoxelMeshAsset;
use shared::simulation::chunk::{
    OpaqueMeshComponent, TransformComponent, TransparentMeshComponent,
};

/// A system that extracts `VoxelMeshAsset` modifications from the simulation world
/// and queues them for GPU upload in the render world.
#[instrument(skip_all)]
#[allow(clippy::type_complexity)]
pub fn extract_modified_voxel_meshes(
    // input
    mut events: Extract<MessageReader<AssetEvent<VoxelMeshAsset>>>,
    assets: Extract<Res<Assets<VoxelMeshAsset>>>,
    opaque_meshes: Extract<
        Query<
            (&OpaqueMeshComponent, &TransformComponent),
            Or<(Added<OpaqueMeshComponent>, Changed<OpaqueMeshComponent>)>,
        >,
    >,
    transparent_meshes: Extract<
        Query<
            (&TransparentMeshComponent, &TransformComponent),
            Or<(
                Added<TransparentMeshComponent>,
                Changed<TransparentMeshComponent>,
            )>,
        >,
    >,
    // output
    mut upload_queue: ResMut<MeshesToUploadQueue>,
) {
    // 1. Process mesh additions and modifications via component change detection
    for (mesh, transform) in opaque_meshes.iter() {
        if let Some(asset) = assets.get(&mesh.mesh_handle) {
            upload_queue
                .queue
                .push((mesh.mesh_handle.id(), asset.clone(), transform.position));
        }
    }

    for (mesh, transform) in transparent_meshes.iter() {
        if let Some(asset) = assets.get(&mesh.mesh_handle) {
            upload_queue
                .queue
                .push((mesh.mesh_handle.id(), asset.clone(), transform.position));
        }
    }

    // 2. Process events strictly for asset removals
    for event in events.read() {
        if let AssetEvent::Removed { id } = event {
            upload_queue.removals.push(*id);
        }
    }
}
