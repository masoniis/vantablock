pub mod bounding_box_pass;
pub mod opaque_pass;
pub mod shared_resources;
pub mod transparent_pass;

use shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout, TextureArrayBindGroupLayout,
};
pub use shared_resources::{
    CentralCameraViewUniform, EnvironmentUniforms, MAIN_DEPTH_FORMAT, MainDepthTextureResource,
};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::{
    global_extract::RenderCameraResource,
    pipeline::main_passes::{
        bounding_box_pass::WireframeRenderPassPlugin,
        opaque_pass::OpaqueRenderPassPlugin,
        shared_resources::{
            resize_main_depth_texture_system, update_camera_view_buffer_system,
            update_environment_uniform_buffer_system,
        },
        transparent_pass::TransparentRenderPassPlugin,
    },
};
use bevy::app::{App, Plugin};
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::prelude::IntoScheduleConfigs;
use bevy::render::{Render, RenderSystems};

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
                resize_main_depth_texture_system,
                update_camera_view_buffer_system,
                update_environment_uniform_buffer_system,
                shared_resources::prepare_texture_array_system,
            )
                .run_if(resource_exists::<RenderCameraResource>)
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
        app.init_resource::<MainDepthTextureResource>();

        app.init_resource::<CentralCameraViewUniform>();
    }
}
