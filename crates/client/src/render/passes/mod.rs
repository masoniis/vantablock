pub mod gpu_resources;
pub mod main_passes;
pub mod shader_registry;
pub mod shadow_pass;

pub use gpu_resources::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    VantablockNode,
    render::passes::{
        main_passes::PlayerCentricRenderPassPlugin, shadow_pass::ShadowRenderPassPlugin,
    },
};
use bevy::{
    app::{App, Plugin},
    render::render_graph::RenderGraphExt,
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
        app.init_resource::<gpu_resources::ChunkStorageBindGroupLayout>();
        app.init_resource::<gpu_resources::ChunkStorageManager>();
    }
}

/// A plugin responsible ONLY for wiring the edges between nodes in the render graph.
/// This must be added AFTER all pass plugins to ensure all nodes exist.
pub struct RenderGraphEdgesPlugin;

impl Plugin for RenderGraphEdgesPlugin {
    fn build(&self, app: &mut App) {
        // Wire the edges inside the Core3d sub-graph to ensure our custom passes
        // run in the correct order relative to Bevy's native passes.
        app.add_render_graph_edges(
            bevy::core_pipeline::core_3d::graph::Core3d,
            (
                bevy::core_pipeline::core_3d::graph::Node3d::StartMainPass,
                VantablockNode::ShadowPass,
                bevy::core_pipeline::core_3d::graph::Node3d::MainOpaquePass,
            ),
        );

        app.add_render_graph_edges(
            bevy::core_pipeline::core_3d::graph::Core3d,
            (
                bevy::core_pipeline::core_3d::graph::Node3d::MainOpaquePass,
                VantablockNode::OpaquePass,
                VantablockNode::TransparentPass,
                bevy::core_pipeline::core_3d::graph::Node3d::MainTransparentPass,
            ),
        );

        app.add_render_graph_edges(
            bevy::core_pipeline::core_3d::graph::Core3d,
            (
                bevy::core_pipeline::core_3d::graph::Node3d::MainTransparentPass,
                VantablockNode::BoundingBoxPass,
                bevy::core_pipeline::core_3d::graph::Node3d::EndMainPass,
            ),
        );
    }
}
