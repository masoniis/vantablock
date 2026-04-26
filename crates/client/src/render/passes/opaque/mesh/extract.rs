use crate::render::chunk::OpaqueMeshComponent;
use crate::render::chunk::VoxelMeshAsset;
use crate::render::types::RenderTransformComponent;
use bevy::{asset::Handle, ecs::prelude::*, render::Extract, render::sync_world::RenderEntity};
use shared::simulation::chunk::TransformComponent;

// INFO: -------------------------------
//         render app components
// -------------------------------------

/// A component in the render world holding the extracted mesh handle.
#[derive(Component, Clone)]
pub struct OpaqueRenderMeshComponent {
    pub mesh_handle: Handle<VoxelMeshAsset>,
}

// INFO: -----------------------------------
//         main app extraction logic
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
