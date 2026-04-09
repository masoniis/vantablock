use crate::{
    prelude::*,
    render::chunk::{OpaqueMeshComponent, TransparentMeshComponent, VoxelMeshAsset},
    render::resources::world_uniforms::VoxelMesh,
};
use bevy::{
    asset::{AssetEvent, AssetId, Assets},
    ecs::{
        prelude::{Added, Changed, MessageReader, Or, Query, Res, ResMut},
        resource::Resource,
    },
    platform::collections::HashMap,
    render::Extract,
};
use shared::simulation::chunk::TransformComponent;

#[derive(Resource, Default)]
pub struct RenderMeshStorageResource {
    pub meshes: HashMap<AssetId<VoxelMeshAsset>, Arc<VoxelMesh>>,
}

#[derive(Resource, Default)]
pub struct MeshesToUploadQueue {
    pub additions: Vec<(AssetId<VoxelMeshAsset>, VoxelMeshAsset, Vec3)>,
    pub removals: Vec<AssetId<VoxelMeshAsset>>,
}

/// A system that extracts `VoxelMeshAsset` modifications from the simulation world
/// and queues them for GPU upload in the render world.
#[instrument(skip_all)]
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
    // process mesh additions and modifications via component change detection
    for (mesh, transform) in opaque_meshes.iter() {
        if let Some(asset) = assets.get(&mesh.mesh_handle) {
            upload_queue
                .additions
                .push((mesh.mesh_handle.id(), asset.clone(), transform.position));
        }
    }

    for (mesh, transform) in transparent_meshes.iter() {
        if let Some(asset) = assets.get(&mesh.mesh_handle) {
            upload_queue
                .additions
                .push((mesh.mesh_handle.id(), asset.clone(), transform.position));
        }
    }

    // process events for asset removals
    for event in events.read() {
        if let AssetEvent::Removed { id } = event {
            upload_queue.removals.push(*id);
        }
    }
}
