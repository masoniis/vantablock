//! # The Voxel Renderer
//!
//! A compute ray-tracing pipeline on Bevy's `RenderApp`:
//!
//! 1. **Main world** ([`geometry`], [`scene`]): bake the canonical asset library into a GPU geometry
//!    library once, then resolve replicated world objects into pose frames, instance records, and a
//!    scene BVH every frame.
//! 2. **Prepare** ([`gpu`]): upload geometry (slab residency), scene buffers, view uniforms, and
//!    the per-view visibility buffer; on adapters with ray queries, write the scene TLAS's
//!    pose-frame boxes.
//! 3. **Graph** ([`passes`]): build the scene TLAS (hardware broad phase only), primary
//!    visibility, then direct lighting, and with `RenderSettings::temporal_aa` the temporal
//!    resolve and its present pass.
//!
//! The traversal shaders mirror the CPU reference in the `voxel` crate, and the GPU conformance
//! tests hold them to it.

pub mod device;
pub mod environment;
pub mod geometry;
pub mod gpu;
pub mod passes;
pub mod scene;
pub mod settings;
pub mod shaders;

pub use settings::{AccelerationMode, DebugView, LocalHierarchy, RenderSettings};

use crate::render::{
    device::GpuCapabilities,
    environment::ExtractedEnvironment,
    geometry::GeometryLibraryHandle,
    gpu::{buffers, temporal, view},
    passes::{
        direct_lighting::{DirectLightingPipeline, queue_direct_lighting_pipelines},
        primary_visibility::PrimaryVisibilityPipeline,
        temporal::{
            TemporalPresentPipeline, TemporalResolvePipeline, queue_temporal_present_pipelines,
        },
    },
    scene::ExtractedScene,
};
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::ExtractResourcePlugin,
        render_resource::SpecializedRenderPipelines,
        renderer::{RenderAdapterInfo, RenderDevice},
    },
};

/// Marks a 3D camera as rendered by the voxel pipeline.
#[derive(Component, ExtractComponent, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct VoxelCamera;

/// Plugin responsible for attaching the voxel renderer to Bevy's native `RenderApp`.
pub struct VantablockRenderPlugin;

impl Plugin for VantablockRenderPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<VoxelCamera>().add_plugins((
            shaders::VantablockShaderPlugin,
            settings::RenderSettingsPlugin,
            geometry::GeometryPlugin,
            scene::ScenePlugin,
            ExtractComponentPlugin::<VoxelCamera>::default(),
            ExtractResourcePlugin::<RenderSettings>::default(),
            ExtractResourcePlugin::<ExtractedEnvironment>::default(),
            ExtractResourcePlugin::<GeometryLibraryHandle>::default(),
            ExtractResourcePlugin::<ExtractedScene>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("RenderApp not found! Ensure DefaultPlugins are added before this plugin.");
            return;
        };
        render_app
            .add_plugins(passes::VoxelRenderGraphPlugin)
            .init_resource::<SpecializedRenderPipelines<DirectLightingPipeline>>()
            .init_resource::<TemporalPresentPipeline>()
            .init_resource::<SpecializedRenderPipelines<TemporalPresentPipeline>>()
            .add_systems(
                Render,
                (
                    (
                        buffers::prepare_geometry_buffers,
                        buffers::prepare_scene_buffers,
                        (
                            view::prepare_visibility_targets,
                            temporal::prepare_temporal_targets,
                        )
                            .chain(),
                    )
                        .in_set(RenderSystems::PrepareResources),
                    view::prepare_view_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    (
                        queue_direct_lighting_pipelines,
                        queue_temporal_present_pipelines,
                    )
                        .in_set(RenderSystems::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let capabilities = GpuCapabilities::probe(
            render_app.world().resource::<RenderDevice>(),
            render_app.world().resource::<RenderAdapterInfo>(),
        );
        capabilities.log();

        render_app
            .insert_resource(capabilities.clone())
            .init_resource::<buffers::VoxelGpuResources>()
            .init_resource::<PrimaryVisibilityPipeline>()
            .init_resource::<DirectLightingPipeline>()
            .init_resource::<TemporalResolvePipeline>();
        app.insert_resource(capabilities);
    }
}
