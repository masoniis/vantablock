//! The voxel render graph stages, added to Bevy's 3D graph.
//!
//! | Node                | Role                                                                |
//! | ------------------- | ------------------------------------------------------------------- |
//! | `PrimaryVisibility` | one world ray per pixel: frame identity, exact cell, depth, costs   |
//! | `DirectLighting`    | materials, sun/sky light, shadow rays, atmosphere, debug views      |
//!
//! Scene preparation (geometry residency, pose frames, BVHs) happens in the render `Prepare`
//! systems before the graph runs. Indirect lighting, refraction, and temporal filtering are later
//! stages that slot in after `DirectLighting`.

pub mod direct_lighting;
pub mod primary_visibility;

use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    prelude::*,
    render::render_graph::{RenderGraphExt, RenderLabel, ViewNodeRunner},
};

#[derive(RenderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum VoxelNode {
    PrimaryVisibility,
    DirectLighting,
}

/// Wires the voxel nodes into the 3D graph. Added to the render app.
pub struct VoxelRenderGraphPlugin;

impl Plugin for VoxelRenderGraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_render_graph_node::<ViewNodeRunner<primary_visibility::PrimaryVisibilityNode>>(
            Core3d,
            VoxelNode::PrimaryVisibility,
        )
        .add_render_graph_node::<ViewNodeRunner<direct_lighting::DirectLightingNode>>(
            Core3d,
            VoxelNode::DirectLighting,
        )
        .add_render_graph_edges(
            Core3d,
            (
                Node3d::MainOpaquePass,
                VoxelNode::PrimaryVisibility,
                VoxelNode::DirectLighting,
                Node3d::MainTransparentPass,
            ),
        );
    }
}
