pub mod extract;
pub mod gpu_resources;
pub mod queue;
pub mod render;

pub use render::BoundingBoxNode;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    VantablockNode, render::passes::main_passes::bounding_box_pass::queue::queue_wireframe_system,
};
use bevy::app::{App, Plugin};
use bevy::prelude::IntoScheduleConfigs;
use bevy::render::render_graph::{RenderGraphExt, ViewNodeRunner};
use bevy::render::{Render, RenderSystems};
use gpu_resources::{
    UnitCubeMesh, WireframeObjectBuffer, WireframePipeline,
    object_binding::WireframeObjectBindGroupLayout,
};

pub struct WireframeRenderPassPlugin;

impl Plugin for WireframeRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(Render, queue_wireframe_system.in_set(RenderSystems::Queue));

        // INFO: -----------------------------------------
        //         Render Graph Integration
        // -----------------------------------------------

        app.add_render_graph_node::<ViewNodeRunner<BoundingBoxNode>>(
            bevy::core_pipeline::core_3d::graph::Core3d,
            VantablockNode::BoundingBoxPass,
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         startup
        // -----------------------

        app.init_resource::<WireframeObjectBindGroupLayout>();
        app.init_resource::<WireframeObjectBuffer>();
        app.init_resource::<WireframePipeline>();
        app.init_resource::<UnitCubeMesh>();
    }
}
