pub mod gpu_resources;
pub mod prepare;
pub mod render;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::passes::shadow::prepare::update_shadow_view_buffer_system;
use crate::render::passes::shadow::render::shadow_pass;
use bevy::app::{App, Plugin};
use bevy::core_pipeline::{Core3d, Core3dSystems, core_3d::main_opaque_pass_3d};
use bevy::prelude::IntoScheduleConfigs;
use bevy::render::{Render, RenderSystems};
use gpu_resources::{
    ShadowDepthTextureResource, ShadowPassPipeline, ShadowViewBuffer,
    shadow_view_uniform::ShadowViewBindGroupLayout,
};

pub struct ShadowRenderPassPlugin;

impl Plugin for ShadowRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------
        //         prepare
        // -----------------------

        app.add_systems(
            Render,
            update_shadow_view_buffer_system.in_set(RenderSystems::Prepare),
        );

        // INFO: ---------------------------------
        //         camera schedule wiring
        // ---------------------------------------

        // the shadow map is rendered before the main opaque pass samples it
        app.add_systems(
            Core3d,
            shadow_pass
                .before(main_opaque_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }

    fn finish(&self, app: &mut App) {
        // set up the render node
        app.init_resource::<ShadowViewBindGroupLayout>();
        app.init_resource::<ShadowPassPipeline>();
        app.init_resource::<ShadowViewBuffer>();
        app.init_resource::<ShadowDepthTextureResource>();
    }
}
