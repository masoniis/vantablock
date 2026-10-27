pub mod mesh;
pub mod render;
pub mod skybox;

pub use mesh::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::{
    VantablockNode,
    passes::opaque::{
        mesh::pipeline::Opaque3dPipeline, render::OpaquePassRenderNode,
        skybox::OpaqueSkyboxPipeline,
    },
};
use bevy::{
    prelude::{App, IntoScheduleConfigs, Plugin},
    render::{
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_resource::SpecializedRenderPipelines,
        {Render, RenderSystems},
    },
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
        //         startup
        // -----------------------

        app.init_resource::<Opaque3dPipeline>();
        app.init_resource::<SpecializedRenderPipelines<Opaque3dPipeline>>();
        app.init_resource::<OpaqueSkyboxPipeline>();
        app.init_resource::<SpecializedRenderPipelines<OpaqueSkyboxPipeline>>();
    }
}
