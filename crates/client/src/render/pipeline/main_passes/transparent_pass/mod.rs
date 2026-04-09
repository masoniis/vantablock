pub mod extract;
pub mod prepare;
pub mod queue;
pub mod render;
pub mod startup;

pub use render::TransparentPassRenderNode;
use startup::TransparentPipeline;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    VantablockNode,
    render::pipeline::main_passes::transparent_pass::{
        prepare::prepare_transparent_meshes_system, queue::queue_and_prepare_transparent_system,
    },
};
use bevy::prelude::IntoScheduleConfigs;
use bevy::{
    app::{App, Plugin},
    render::render_graph::{RenderGraphExt, ViewNodeRunner},
    render::render_resource::SpecializedRenderPipelines,
    render::{Render, RenderSystems},
};

pub struct TransparentRenderPassPlugin;

impl Plugin for TransparentRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------
        //         prepare
        // -----------------------

        app.add_systems(
            Render,
            prepare_transparent_meshes_system.in_set(RenderSystems::Prepare),
        );

        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(
            Render,
            queue_and_prepare_transparent_system.in_set(RenderSystems::Queue),
        );

        // INFO: ----------------------------------
        //         render graph integration
        // ----------------------------------------

        app.add_render_graph_node::<ViewNodeRunner<TransparentPassRenderNode>>(
            bevy::core_pipeline::core_3d::graph::Core3d,
            VantablockNode::TransparentPass,
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         startup
        // -----------------------

        app.init_resource::<TransparentPipeline>();
        app.init_resource::<SpecializedRenderPipelines<TransparentPipeline>>();
    }
}
