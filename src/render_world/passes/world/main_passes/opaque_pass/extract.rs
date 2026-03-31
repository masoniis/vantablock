use crate::{
    render_world::{
        global_extract::{extract_resource::ExtractResource, MirrorableComponent},
        passes::world::main_passes::opaque_pass::startup::OpaqueRenderMode,
    },
    simulation_world::{
        asset_management::{asset_storage::Handle, MeshAsset},
        chunk::{OpaqueMeshComponent, TransformComponent},
        input::systems::toggle_opaque_wireframe::OpaqueWireframeMode,
    },
};
use bevy::math::Mat4;
use bevy_ecs::prelude::*;

// INFO: --------------------------------
//         RenderWorld components
// --------------------------------------

/// A component in the render world holding the extracted mesh handle.
#[derive(Component, Clone)]
pub struct OpaqueRenderMeshComponent {
    pub mesh_handle: Handle<MeshAsset>,
}

/// A component representing a transform on a mesh
#[derive(Component, Clone)]
pub struct RenderTransformComponent {
    pub transform: Mat4,
}

// INFO: -----------------------------------
//         SimWorld extraction logic
// -----------------------------------------

pub struct OpaqueRenderModeExtractor;

/// Extract the current opaque render mode (wireframe or fill) from the simulation world
impl ExtractResource for OpaqueRenderModeExtractor {
    type Source = OpaqueWireframeMode;
    type Output = OpaqueRenderMode;

    fn extract_and_update(
        commands: &mut Commands,
        source: &Self::Source,
        target: Option<ResMut<Self::Output>>,
    ) {
        let new_mode = if source.enabled {
            OpaqueRenderMode::Wireframe
        } else {
            OpaqueRenderMode::Fill
        };

        if let Some(mut target) = target {
            if *target != new_mode {
                *target = new_mode;
            }
        } else {
            commands.insert_resource(new_mode);
        }
    }
}

/// Mirror properties of `MeshComponent` from the simulation world
impl MirrorableComponent for OpaqueMeshComponent {
    type Dependencies = &'static TransformComponent;
    type RenderBundle = (OpaqueRenderMeshComponent, RenderTransformComponent);

    type Filter = Or<(
        Added<OpaqueMeshComponent>,
        Changed<OpaqueMeshComponent>,
        Changed<TransformComponent>,
    )>;

    fn to_render_bundle(&self, transform: &TransformComponent) -> Self::RenderBundle {
        (
            OpaqueRenderMeshComponent {
                mesh_handle: self.mesh_handle,
            },
            RenderTransformComponent {
                transform: transform.to_matrix(),
            },
        )
    }
}
