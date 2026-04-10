pub mod bounding_box;
pub mod opaque;
pub mod shadow;
pub mod transparent;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    VantablockNode,
    render::{
        passes::{
            bounding_box::WireframeRenderPassPlugin, opaque::OpaqueRenderPassPlugin,
            shadow::ShadowRenderPassPlugin, transparent::TransparentRenderPassPlugin,
        },
        resources::{
            CentralCameraViewBindGroupLayout, CentralCameraViewUniform, EnvironmentBindGroupLayout,
            EnvironmentUniforms, TextureArrayBindGroupLayout, prepare_texture_array_system,
            update_camera_view_buffer_system, update_environment_uniform_buffer_system,
            world_uniforms::{ChunkStorageBindGroupLayout, ChunkStorageManager},
        },
    },
};
use bevy::prelude::IntoScheduleConfigs;
use bevy::{
    app::{App, Plugin},
    render::{Render, RenderSystems, render_graph::RenderGraphExt},
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
        app.init_resource::<ChunkStorageManager>();
    }
}

/// A plugin responsible ONLY for wiring the edges between nodes in the render graph.
/// This must be added AFTER all pass plugins to ensure all nodes exist.
pub struct RenderGraphEdgesPlugin;

impl Plugin for RenderGraphEdgesPlugin {
    fn build(&self, app: &mut App) {
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
