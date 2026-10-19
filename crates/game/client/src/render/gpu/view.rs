//! Per-view state: the visibility buffer targets, the view uniform, and the view bind groups.
//!
//! The visibility buffer is the spec's opaque-hit record: resolved pose-frame identity and the
//! hit voxel's attribute handle (`Rg32Uint`), world-ray distance (`R32Float`), the exact integer
//! group-local cell (`R32Uint`, kept separately because a DAG handle does not identify a spatial
//! occurrence), and per-pixel traversal counters (`R32Uint`).

use crate::render::{
    VoxelCamera, environment::ExtractedEnvironment, gpu::layout::*, settings::RenderSettings,
};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BufferInitDescriptor, BufferUsages, Extent3d, PipelineCache, ShaderStages,
            StorageTextureAccess, TextureDescriptor, TextureDimension, TextureFormat,
            TextureSampleType, TextureUsages,
            binding_types::{texture_2d, texture_storage_2d, uniform_buffer_sized},
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
        view::ExtractedView,
    },
};

pub const VISIBILITY_FORMAT: TextureFormat = TextureFormat::Rg32Uint;
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::R32Float;
pub const CELL_FORMAT: TextureFormat = TextureFormat::R32Uint;
pub const STATS_FORMAT: TextureFormat = TextureFormat::R32Uint;

pub fn visibility_view_layout() -> BindGroupLayoutDescriptor {
    let write = StorageTextureAccess::WriteOnly;
    BindGroupLayoutDescriptor::new(
        "voxel_visibility_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer_sized(false, None),
                texture_storage_2d(VISIBILITY_FORMAT, write),
                texture_storage_2d(DEPTH_FORMAT, write),
                texture_storage_2d(CELL_FORMAT, write),
                texture_storage_2d(STATS_FORMAT, write),
            ),
        ),
    )
}

pub fn lighting_view_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_lighting_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer_sized(false, None),
                uniform_buffer_sized(false, None),
                texture_2d(TextureSampleType::Uint),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_2d(TextureSampleType::Uint),
                texture_2d(TextureSampleType::Uint),
            ),
        ),
    )
}

/// The visibility buffer of one view.
#[derive(Component)]
pub struct VisibilityTargets {
    /// The traced ray grid size.
    pub size: UVec2,
    /// The view target size it is upsampled to.
    pub target: UVec2,
    pub visibility: CachedTexture,
    pub depth: CachedTexture,
    pub cell: CachedTexture,
    pub stats: CachedTexture,
}

#[derive(Component)]
pub struct VoxelViewBindGroups {
    pub visibility: BindGroup,
    pub lighting: BindGroup,
}

/// Allocates visibility targets for every voxel camera view.
pub fn prepare_visibility_targets(
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    settings: Option<Res<RenderSettings>>,
    views: Query<(Entity, &ExtractedCamera), With<VoxelCamera>>,
) {
    let scale = settings.map_or(1.0, |s| s.render_scale.clamp(0.1, 1.0));
    for (entity, camera) in &views {
        let Some(target) = camera.physical_viewport_size.filter(|s| s.x > 0 && s.y > 0) else {
            continue;
        };
        let size = (target.as_vec2() * scale)
            .round()
            .as_uvec2()
            .max(UVec2::ONE);
        let mut texture = |label: &'static str, format: TextureFormat| {
            texture_cache.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: size.x,
                        height: size.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            )
        };
        let targets = VisibilityTargets {
            size,
            target,
            visibility: texture("voxel_visibility", VISIBILITY_FORMAT),
            depth: texture("voxel_depth", DEPTH_FORMAT),
            cell: texture("voxel_cell", CELL_FORMAT),
            stats: texture("voxel_stats", STATS_FORMAT),
        };
        commands.entity(entity).insert(targets);
    }
}

/// Writes the view uniform and builds the per-view bind groups.
#[allow(clippy::too_many_arguments)]
pub fn prepare_view_bind_groups(
    mut previous_clip_from_world: Local<HashMap<Entity, Mat4>>,
    mut commands: Commands,
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    settings: Option<Res<RenderSettings>>,
    environment: Option<Res<ExtractedEnvironment>>,
    views: Query<(Entity, &ExtractedView, &ExtractedCamera, &VisibilityTargets), With<VoxelCamera>>,
) {
    let debug_view = settings.map_or(0, |s| s.debug_view.index());
    let environment = environment.map(|e| e.0).unwrap_or_default();
    let visibility_layout = pipeline_cache.get_bind_group_layout(&visibility_view_layout());
    let lighting_layout = pipeline_cache.get_bind_group_layout(&lighting_view_layout());

    let mut seen = HashMap::new();
    for (entity, view, camera, targets) in &views {
        let view_from_world = view.world_from_view.to_matrix().inverse();
        let clip_from_world = view.clip_from_view * view_from_world;
        let prev_clip_from_world = previous_clip_from_world
            .get(&entity)
            .copied()
            .unwrap_or(clip_from_world);
        seen.insert(entity, clip_from_world);
        let uniform = GpuViewUniform {
            world_from_clip: clip_from_world.inverse().to_cols_array(),
            clip_from_world: clip_from_world.to_cols_array(),
            prev_clip_from_world: prev_clip_from_world.to_cols_array(),
            camera_position: view.world_from_view.translation().to_array(),
            debug_view,
            viewport: targets.size.as_vec2().to_array(),
            // Bevy tonemaps HDR cameras itself
            flags: if camera.hdr {
                0
            } else {
                VIEW_TONEMAP_IN_SHADER
            },
            target_to_grid: targets.size.x as f32 / targets.target.x as f32,
        };
        let view_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("voxel_view_uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: BufferUsages::UNIFORM,
        });
        let environment_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("voxel_environment_uniform"),
            contents: bytemuck::bytes_of(&environment),
            usage: BufferUsages::UNIFORM,
        });

        let visibility = device.create_bind_group(
            "voxel_visibility_view_bind_group",
            &visibility_layout,
            &BindGroupEntries::sequential((
                view_buffer.as_entire_binding(),
                &targets.visibility.default_view,
                &targets.depth.default_view,
                &targets.cell.default_view,
                &targets.stats.default_view,
            )),
        );
        let lighting = device.create_bind_group(
            "voxel_lighting_view_bind_group",
            &lighting_layout,
            &BindGroupEntries::sequential((
                view_buffer.as_entire_binding(),
                environment_buffer.as_entire_binding(),
                &targets.visibility.default_view,
                &targets.depth.default_view,
                &targets.cell.default_view,
                &targets.stats.default_view,
            )),
        );
        commands.entity(entity).insert(VoxelViewBindGroups {
            visibility,
            lighting,
        });
    }
    // views that disappeared are forgotten, so a reused entity starts without stale history
    *previous_clip_from_world = seen;
}
