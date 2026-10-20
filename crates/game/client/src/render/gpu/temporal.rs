//! Per-view state of the temporal resolve (`RenderSettings::temporal_aa`): the jitter sequence,
//! lighting's grid-resolution outputs, and the two history images the resolve alternates between.
//!
//! With the resolve on, primary visibility traces each pixel's ray through a Halton (2, 3)
//! subpixel offset, lighting shades the traced grid into [`TemporalTargets::current`] and writes
//! where each pixel's surface was last frame into [`TemporalTargets::offsets`], and
//! `temporal_resolve.wesl` blends the reprojected history into the other history image, which the
//! present pass shows in the view target (see `passes::temporal`).

use crate::render::{VoxelCamera, gpu::view::VisibilityTargets, settings::RenderSettings};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayoutDescriptor, BindGroupLayoutEntries, Extent3d, SamplerBindingType,
            ShaderStages, StorageTextureAccess, Texture, TextureDescriptor, TextureDimension,
            TextureFormat, TextureSampleType, TextureUsages, TextureView,
            binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer_sized},
        },
        renderer::RenderDevice,
        texture::{CachedTexture, TextureCache},
    },
};

/// Lighting's shaded grid and the resolve's history.
pub const COLOR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// Where each pixel's surface was last frame, as an offset in traced pixels.
pub const OFFSET_FORMAT: TextureFormat = TextureFormat::Rg16Float;
/// Frames in the jitter sequence before it repeats.
const JITTER_PHASES: u32 = 16;

pub fn resolve_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_temporal_resolve_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer_sized(false, None),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_storage_2d(COLOR_FORMAT, StorageTextureAccess::WriteOnly),
            ),
        ),
    )
}

pub fn present_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "voxel_temporal_present_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer_sized(false, None),
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        ),
    )
}

/// The `index`th point (from 1) of the Halton sequence in `base`, in `[0, 1)`.
fn halton(mut index: u32, base: u32) -> f32 {
    let (mut result, mut f) = (0.0, 1.0);
    while index > 0 {
        f /= base as f32;
        result += f * (index % base) as f32;
        index /= base;
    }
    result
}

/// The subpixel offset of every primary ray in a view's `frame`th temporal frame, in traced
/// pixels within `[-0.5, 0.5)`: the Halton (2, 3) sequence, which covers the pixel evenly, so the
/// history converges to the pixel's box-filtered average.
pub fn jitter(frame: u32) -> Vec2 {
    let i = frame % JITTER_PHASES + 1;
    Vec2::new(halton(i, 2), halton(i, 3)) - 0.5
}

/// A view's temporal resolve state for this frame.
#[derive(Component)]
pub struct TemporalTargets {
    /// Lighting's shaded grid (render attachment).
    pub current: CachedTexture,
    /// Lighting's history offsets (render attachment).
    pub offsets: CachedTexture,
    /// Last frame's resolved image, which the resolve reprojects.
    pub history: TextureView,
    /// This frame's resolved image, which the present pass shows and the next frame reprojects.
    pub resolved: TextureView,
    /// Whether `history` holds last frame's resolve of a grid this size.
    pub history_valid: bool,
    pub jitter: Vec2,
}

/// The two history images of a view, alternating each frame.
pub struct History {
    size: UVec2,
    textures: [(Texture, TextureView); 2],
    /// Frames resolved into them so far.
    frame: u32,
}

/// Allocates the temporal targets of every voxel camera view while `RenderSettings::temporal_aa`
/// is on, and drops a view's history when it is off, resized, or gone.
pub fn prepare_temporal_targets(
    mut histories: Local<HashMap<Entity, History>>,
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    settings: Option<Res<RenderSettings>>,
    views: Query<(Entity, &VisibilityTargets), With<VoxelCamera>>,
) {
    let enabled = settings.is_some_and(|s| s.temporal_aa);
    let mut seen = HashMap::new();
    for (entity, targets) in &views {
        if !enabled {
            commands.entity(entity).remove::<TemporalTargets>();
            continue;
        }
        let size = targets.size;
        let descriptor = |label: &'static str, format, usage| TextureDescriptor {
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
            usage,
            view_formats: &[],
        };
        let mut history = histories
            .remove(&entity)
            .filter(|h| h.size == size)
            .unwrap_or_else(|| History {
                size,
                textures: std::array::from_fn(|_| {
                    let texture = device.create_texture(&descriptor(
                        "voxel_temporal_history",
                        COLOR_FORMAT,
                        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                    ));
                    let view = texture.create_view(&Default::default());
                    (texture, view)
                }),
                frame: 0,
            });
        let attachment = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        let write = (history.frame % 2) as usize;
        commands.entity(entity).insert(TemporalTargets {
            current: texture_cache.get(
                &device,
                descriptor("voxel_temporal_current", COLOR_FORMAT, attachment),
            ),
            offsets: texture_cache.get(
                &device,
                descriptor("voxel_temporal_offsets", OFFSET_FORMAT, attachment),
            ),
            history: history.textures[1 - write].1.clone(),
            resolved: history.textures[write].1.clone(),
            history_valid: history.frame > 0,
            jitter: jitter(history.frame),
        });
        history.frame = history.frame.wrapping_add(1);
        seen.insert(entity, history);
    }
    *histories = seen;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_covers_the_pixel_evenly() {
        let offsets: Vec<Vec2> = (0..JITTER_PHASES).map(jitter).collect();
        assert!(
            offsets
                .iter()
                .all(|o| o.cmpge(Vec2::splat(-0.5)).all() && o.cmplt(Vec2::splat(0.5)).all())
        );
        let mean = offsets.iter().sum::<Vec2>() / JITTER_PHASES as f32;
        assert!(mean.abs().max_element() < 0.05, "mean offset {mean}");
        // every quadrant of the pixel gets about a quarter of the samples
        for quadrant in 0..4 {
            let inside = offsets
                .iter()
                .filter(|o| (o.x >= 0.0) == (quadrant & 1 == 1) && (o.y >= 0.0) == (quadrant >= 2))
                .count();
            assert!(
                (3..=5).contains(&inside),
                "quadrant {quadrant} holds {inside}"
            );
        }
    }
}
