use crate::{
    prelude::*,
    render::chunk::{BlockMeshAsset, OpaqueMeshComponent, TransparentMeshComponent},
    render::resources::world_uniforms::BlockMesh,
};
use bevy::{
    asset::{AssetEvent, AssetId, Assets},
    ecs::{
        prelude::{Added, Changed, MessageReader, Or, Query, Res, ResMut},
        resource::Resource,
    },
    platform::collections::HashMap,
    render::Extract,
    transform::components::Transform,
};

#[derive(Resource, Default)]
pub struct RenderMeshStorageResource {
    pub meshes: HashMap<AssetId<BlockMeshAsset>, Arc<BlockMesh>>,
}

#[derive(Resource, Default)]
pub struct MeshesToUploadQueue {
    pub additions: Vec<(AssetId<BlockMeshAsset>, BlockMeshAsset, Vec3)>,
    pub removals: Vec<AssetId<BlockMeshAsset>>,
}

/// A system that extracts `BlockMeshAsset` modifications from the simulation world
/// and queues them for GPU upload in the render world.
#[instrument(skip_all)]
pub fn extract_modified_block_meshes(
    // input
    mut events: Extract<MessageReader<AssetEvent<BlockMeshAsset>>>,
    assets: Extract<Res<Assets<BlockMeshAsset>>>,
    opaque_meshes: Extract<
        Query<
            (&OpaqueMeshComponent, &Transform),
            Or<(Added<OpaqueMeshComponent>, Changed<OpaqueMeshComponent>)>,
        >,
    >,
    transparent_meshes: Extract<
        Query<
            (&TransparentMeshComponent, &Transform),
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
            upload_queue.additions.push((
                mesh.mesh_handle.id(),
                asset.clone(),
                transform.translation,
            ));
        }
    }

    for (mesh, transform) in transparent_meshes.iter() {
        if let Some(asset) = assets.get(&mesh.mesh_handle) {
            upload_queue.additions.push((
                mesh.mesh_handle.id(),
                asset.clone(),
                transform.translation,
            ));
        }
    }

    // process events for asset removals
    for event in events.read() {
        if let AssetEvent::Removed { id } = event {
            upload_queue.removals.push(*id);
        }
    }
}
