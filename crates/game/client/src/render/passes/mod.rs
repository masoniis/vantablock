pub mod bounding_box;
pub mod opaque;
pub mod shadow;
pub mod transparent;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::{
    passes::{
        bounding_box::WireframeRenderPassPlugin, opaque::OpaqueRenderPassPlugin,
        shadow::ShadowRenderPassPlugin, transparent::TransparentRenderPassPlugin,
    },
    resources::{
        CentralCameraViewBindGroupLayout, CentralCameraViewUniform, EnvironmentBindGroupLayout,
        EnvironmentUniforms, TextureArrayBindGroupLayout, prepare_texture_array_system,
        update_camera_view_buffer_system, update_environment_uniform_buffer_system,
        world_uniforms::{ChunkGpuStorage, ChunkStorageBindGroupLayout},
    },
};
use bevy::prelude::IntoScheduleConfigs;
use bevy::{
    app::{App, Plugin},
    render::{Render, RenderSystems},
};

/// A plugin that sets up all the necessary resources and render
/// passes used in the rendering pipeline.
pub struct WorldRenderPassesPlugin;

impl Plugin for WorldRenderPassesPlugin {
    fn build(&self, app: &mut App) {
        // renderpass plugins
        app.add_plugins((ShadowRenderPassPlugin, PlayerCentricRenderPassPlugin));
    }

    fn finish(&self, app: &mut App) {
        // shared world uniform resources
        app.init_resource::<ChunkStorageBindGroupLayout>();
        app.init_resource::<ChunkGpuStorage>();
    }
}

/// A plugin that sets up all the necessary resources and render
/// passes used in the rendering pipeline.
pub struct PlayerCentricRenderPassPlugin;

impl Plugin for PlayerCentricRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------------------------------
        //         prepare (also shared resources)
        // -----------------------------------------------

        app.add_systems(
            Render,
            (
                update_camera_view_buffer_system,
                update_environment_uniform_buffer_system,
                prepare_texture_array_system,
            )
                .in_set(RenderSystems::Prepare),
        );

        // INFO: --------------------------------------
        //         subplugins for render passes
        // --------------------------------------------

        app.add_plugins((
            TransparentRenderPassPlugin,
            OpaqueRenderPassPlugin,
            WireframeRenderPassPlugin,
        ));
    }

    fn finish(&self, app: &mut App) {
        // INFO: ----------------------------------------------------
        //         startup (shared resources for main passes)
        // ----------------------------------------------------------

        app.init_resource::<CentralCameraViewBindGroupLayout>();
        app.init_resource::<EnvironmentBindGroupLayout>();
        app.init_resource::<EnvironmentUniforms>();
        app.init_resource::<TextureArrayBindGroupLayout>();

        app.init_resource::<CentralCameraViewUniform>();
    }
}
