pub mod extract;
pub mod prepare;
pub mod queue;
pub mod render;
pub mod startup;

pub use render::OpaquePassRenderNode;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::VantablockNode;
use bevy::app::{App, Plugin};
use bevy::prelude::IntoScheduleConfigs;
use bevy::render::render_graph::{RenderGraphExt, ViewNodeRunner};
use bevy::render::{Render, RenderSystems};
use startup::OpaquePipelines;

pub struct OpaqueRenderPassPlugin;

impl Plugin for OpaqueRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------
        //         prepare
        // -----------------------

        app.add_systems(
            Render,
            prepare::prepare_opaque_meshes_system.in_set(RenderSystems::Prepare),
        );

        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(
            Render,
            queue::queue_opaque_system.in_set(RenderSystems::Queue),
        );

        // INFO: -----------------------------------------
        //         render graph integration
        // -----------------------------------------------

        app.add_render_graph_node::<ViewNodeRunner<OpaquePassRenderNode>>(
            bevy::core_pipeline::core_3d::graph::Core3d,
            VantablockNode::OpaquePass,
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         Startup
        // -----------------------

        app.init_resource::<OpaquePipelines>();
    }
}
