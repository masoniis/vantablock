pub mod extract;
pub mod pipeline;
pub mod queue;
pub mod render;

pub use pipeline::*;
pub use render::OpaquePassRenderNode;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::VantablockNode;
use bevy::prelude::{App, IntoScheduleConfigs, Plugin};
use bevy::render::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_resource::SpecializedRenderPipelines,
    {Render, RenderSystems},
};

pub struct OpaqueRenderPassPlugin;

impl Plugin for OpaqueRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(
            Render,
            queue::queue_opaque_system.in_set(RenderSystems::Queue),
        );

        // INFO: ----------------------------------
        //         render graph integration
        // ----------------------------------------

        app.add_render_graph_node::<ViewNodeRunner<OpaquePassRenderNode>>(
            bevy::core_pipeline::core_3d::graph::Core3d,
            VantablockNode::OpaquePass,
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         Startup
        // -----------------------

        app.init_resource::<WorldOpaquePipeline>();
        app.init_resource::<SpecializedRenderPipelines<WorldOpaquePipeline>>();
    }
}
