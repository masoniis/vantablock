use crate::render::types::RenderTransformComponent;
use bevy::asset::Handle;
use bevy::ecs::prelude::*;
use bevy::render::Extract;
use bevy::render::sync_world::RenderEntity;
use shared::simulation::asset::VoxelMeshAsset;
use shared::simulation::chunk::{OpaqueMeshComponent, TransformComponent};

// INFO: --------------------------------
//         RenderWorld components
// --------------------------------------

/// A component in the render world holding the extracted mesh handle.
#[derive(Component, Clone)]
pub struct OpaqueRenderMeshComponent {
    pub mesh_handle: Handle<VoxelMeshAsset>,
}

// INFO: -----------------------------------
//         SimWorld extraction logic
// -----------------------------------------

/// A system that extracts opaque meshes from the simulation world into the render world.
pub fn extract_opaque_meshes(
    mut commands: Commands,
    query: Extract<Query<(&RenderEntity, &OpaqueMeshComponent, &TransformComponent)>>,
) {
    for (render_entity, mesh, transform) in query.iter() {
        commands.entity(render_entity.id()).insert((
            OpaqueRenderMeshComponent {
                mesh_handle: mesh.mesh_handle.clone(),
            },
            RenderTransformComponent {
                transform: transform.to_matrix(),
            },
        ));
    }
}
