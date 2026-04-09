use crate::render::chunk::{TransparentMeshComponent, VoxelMeshAsset};
use crate::render::types::RenderTransformComponent;
use bevy::asset::Handle;
use bevy::ecs::prelude::*;
use bevy::render::Extract;
use bevy::render::sync_world::RenderEntity;
use shared::simulation::chunk::TransformComponent;

// INFO: -------------------------------
//         render app components
// -------------------------------------

/// A component in the render world holding the extracted mesh handle.
#[derive(Component, Clone)]
pub struct TransparentRenderMeshComponent {
    pub mesh_handle: Handle<VoxelMeshAsset>,
}

// INFO: -----------------------------------
//         main app extraction logic
// -----------------------------------------

/// A system that extracts transparent meshes from the simulation world into the render world.
pub fn extract_transparent_meshes(
    mut commands: Commands,
    query: Extract<
        Query<(
            &RenderEntity,
            &TransparentMeshComponent,
            &TransformComponent,
        )>,
    >,
) {
    for (render_entity, mesh, transform) in query.iter() {
        commands.entity(render_entity.id()).insert((
            TransparentRenderMeshComponent {
                mesh_handle: mesh.mesh_handle.clone(),
            },
            RenderTransformComponent {
                transform: transform.to_matrix(),
            },
        ));
    }
}
