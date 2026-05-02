use crate::render::chunk::BlockMeshAsset;
use crate::render::chunk::OpaqueMeshComponent;
use bevy::{
    asset::Handle, ecs::prelude::*, render::Extract, render::sync_world::RenderEntity,
    transform::components::GlobalTransform,
};

// INFO: -------------------------------
//         render app components
// -------------------------------------

/// A component in the render world holding the extracted mesh handle.
#[derive(Component, Clone)]
pub struct OpaqueRenderMeshComponent {
    pub mesh_handle: Handle<BlockMeshAsset>,
}

// INFO: -----------------------------------
//         main app extraction logic
// -----------------------------------------

/// A system that extracts opaque meshes from the simulation world into the render world.
pub fn extract_opaque_meshes(
    mut commands: Commands,
    query: Extract<Query<(&RenderEntity, &OpaqueMeshComponent, &GlobalTransform)>>,
) {
    for (render_entity, mesh, transform) in query.iter() {
        commands.entity(render_entity.id()).insert((
            OpaqueRenderMeshComponent {
                mesh_handle: mesh.mesh_handle.clone(),
            },
            *transform,
        ));
    }
}
