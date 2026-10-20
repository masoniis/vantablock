//! The voxel render passes, added to each camera's `Core3d` schedule.
//!
//! | Pass                | Role                                                                |
//! | ------------------- | ------------------------------------------------------------------- |
//! | `SceneTlas`         | builds the hardware broad phase's TLAS (adapters with ray queries)  |
//! | `PrimaryVisibility` | one world ray per pixel: frame identity, exact cell, depth, costs   |
//! | `DirectLighting`    | materials, sun/sky light, shadow rays, atmosphere, debug views      |
//! | `TemporalResolve`   | jittered frames blended with reprojected history (`temporal_aa`)    |
//! | `TemporalPresent`   | the resolved image into the view target (`temporal_aa`)             |
//!
//! Scene preparation (geometry residency, pose frames, BVHs) happens in the render `Prepare`
//! systems before the camera schedules run. Without the temporal resolve, `DirectLighting` writes
//! the view target itself. Indirect lighting and refraction are later stages that slot in after
//! `DirectLighting`.

pub mod direct_lighting;
pub mod primary_visibility;
pub mod temporal;

use crate::render::gpu::broad_phase;
use bevy::{
    core_pipeline::{Core3d, Core3dSystems},
    prelude::*,
};

/// Wires the voxel passes into the 3D camera schedule, after Bevy's opaque pass and before its
/// transparent pass, which share `Core3dSystems::MainPass`. Added to the render app.
pub struct VoxelRenderGraphPlugin;

impl Plugin for VoxelRenderGraphPlugin {
    fn build(&self, app: &mut App) {
        use bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d};
        app.add_systems(
            Core3d,
            (
                broad_phase::build_scene_tlas,
                primary_visibility::primary_visibility_pass,
                direct_lighting::direct_lighting_pass,
                temporal::temporal_resolve_pass,
                temporal::temporal_present_pass,
            )
                .chain()
                .after(main_opaque_pass_3d)
                .before(main_transparent_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }
}
