pub mod gpu_resources;
pub mod prepare;
pub mod render;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::pipeline::shadow_pass::prepare::update_shadow_view_buffer_system;
use crate::render::pipeline::shadow_pass::render::ShadowRenderPassNode;
use crate::render::scheduling::VantablockNode;
use bevy::app::{App, Plugin};
use bevy::prelude::IntoScheduleConfigs;
use bevy::render::render_graph::{RenderGraphExt, ViewNodeRunner};
use bevy::render::{Render, RenderSystems};
use gpu_resources::{
    shadow_view_uniform::ShadowViewBindGroupLayout, ShadowDepthTextureResource, ShadowPassPipeline,
    ShadowViewBuffer,
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

        // INFO: ----------------------------------
        //         render graph integration
        // ----------------------------------------

        app.add_render_graph_node::<ViewNodeRunner<ShadowRenderPassNode>>(
            bevy::core_pipeline::core_3d::graph::Core3d,
            VantablockNode::ShadowPass,
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
