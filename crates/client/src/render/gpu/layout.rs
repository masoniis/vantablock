//! GPU-facing data layouts, mirrored field for field in
//! `assets/client/shader/voxel/lib/types.wesl`. Every struct here is `Pod` and its size is
//! asserted, so a layout drift fails to compile instead of corrupting the GPU's view.

use bevy::math::Affine3A;
use bytemuck::{Pod, Zeroable};
use voxel::{handle::MAX_PAGES, scene::ResolvedPoseFrame};

/// `PoseFrame::flags`: every topology block of the frame's root lives in one page.
pub const FRAME_SINGLE_PAGE: u32 = 1;
/// `InstanceRecord::flags`: search this instance with its template mini-BVH.
pub const INSTANCE_USES_TEMPLATE: u32 = 1;
/// `InstanceRecord::mini_bvh_root` for single-root assets.
pub const NO_MINI_BVH: u32 = u32::MAX;
/// Visibility buffer value for pixels that hit nothing.
pub const VISIBILITY_MISS: u32 = u32::MAX;

/// `SceneUniform::mode` values.
pub const MODE_TEMPLATE_MINI_BVH: u32 = 0;
pub const MODE_FLATTENED_GROUPS: u32 = 1;

/// `ViewUniform::flags`: the target is not HDR, so the shader tonemaps itself.
pub const VIEW_TONEMAP_IN_SHADER: u32 = 1;

/// An affine transform as three rows of `[x, y, z, translation]`.
pub type AffineRows = [[f32; 4]; 3];

pub fn affine_rows(m: &Affine3A) -> AffineRows {
    let (x, y, z, t) = (
        m.matrix3.x_axis,
        m.matrix3.y_axis,
        m.matrix3.z_axis,
        m.translation,
    );
    [
        [x.x, y.x, z.x, t.x],
        [x.y, y.y, z.y, t.y],
        [x.z, y.z, z.z, t.z],
    ]
}

/// One resolved pose frame: an instance's articulated group, ready to trace.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuPoseFrame {
    pub world_to_local: AffineRows,
    pub local_to_world: AffineRows,
    pub prev_local_to_world: AffineRows,
    pub bounds_min: [f32; 3],
    /// Logical topology handle of the group's root node.
    pub root_node: u32,
    pub bounds_max: [f32; 3],
    /// Slab word base of the root's page, for the single-page fast path.
    pub root_page_base: u32,
    pub levels: u32,
    pub flags: u32,
    pub palette_base: u32,
    pub instance: u32,
}
const _: () = assert!(std::mem::size_of::<GpuPoseFrame>() == 192);

impl GpuPoseFrame {
    pub fn new(
        frame: &ResolvedPoseFrame,
        root: &voxel::Tree64Root,
        root_page_base: u32,
        palette_base: u32,
    ) -> Self {
        Self {
            world_to_local: affine_rows(&frame.world_to_group),
            local_to_world: affine_rows(&frame.group_to_world),
            prev_local_to_world: affine_rows(&frame.prev_group_to_world),
            bounds_min: frame.world_bounds.min.to_array(),
            root_node: root.node.raw(),
            bounds_max: frame.world_bounds.max.to_array(),
            root_page_base,
            levels: root.levels as u32,
            flags: if root.single_page {
                FRAME_SINGLE_PAGE
            } else {
                0
            },
            palette_base,
            instance: frame.instance,
        }
    }
}

/// A world instance, the scene BVH leaf in template mini-BVH mode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuInstance {
    pub world_to_root: AffineRows,
    /// Index of the asset's template mini-BVH root in the template node buffer.
    pub mini_bvh_root: u32,
    pub slot_base: u32,
    pub group_count: u32,
    pub flags: u32,
}
const _: () = assert!(std::mem::size_of::<GpuInstance>() == 64);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuSceneUniform {
    pub frame_count: u32,
    pub instance_count: u32,
    pub bvh_node_count: u32,
    pub mode: u32,
}

/// Logical page to slab word base, for topology and attribute handles. Pages are packed four
/// per `vec4<u32>` because uniform arrays have a 16 byte stride.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuGeometryMeta {
    pub topology_page_base: [[u32; 4]; PAGE_TABLE_VEC4S],
    pub attribute_page_base: [[u32; 4]; PAGE_TABLE_VEC4S],
}
pub const PAGE_TABLE_VEC4S: usize = (MAX_PAGES as usize).div_ceil(4);
const _: () = assert!(std::mem::size_of::<GpuGeometryMeta>() == 2048);

impl GpuGeometryMeta {
    pub fn new(topology: &[u32], attributes: &[u32]) -> Self {
        let pack = |bases: &[u32]| {
            let mut out = [[0u32; 4]; PAGE_TABLE_VEC4S];
            for (page, &base) in bases.iter().enumerate() {
                out[page / 4][page % 4] = base;
            }
            out
        };
        Self {
            topology_page_base: pack(topology),
            attribute_page_base: pack(attributes),
        }
    }
}

/// Per-view camera data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuViewUniform {
    pub world_from_clip: [f32; 16],
    pub clip_from_world: [f32; 16],
    pub camera_position: [f32; 3],
    pub debug_view: u32,
    /// Size of the traced ray grid (the visibility buffer), in pixels.
    pub viewport: [f32; 2],
    pub flags: u32,
    /// Ray grid pixels per view target pixel (the render scale).
    pub target_to_grid: f32,
}
const _: () = assert!(std::mem::size_of::<GpuViewUniform>() == 160);

/// Sun, sky, and atmosphere parameters.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuEnvironment {
    /// Direction toward the main light (sun or moon).
    pub light_direction: [f32; 3],
    pub ambient: f32,
    pub light_color: [f32; 3],
    pub time: f32,
    pub zenith: [f32; 3],
    pub _pad0: f32,
    pub horizon: [f32; 3],
    pub _pad1: f32,
    pub sun_direction: [f32; 3],
    pub _pad2: f32,
    pub sun_disk: [f32; 3],
    pub _pad3: f32,
    pub moon_direction: [f32; 3],
    pub _pad4: f32,
}
const _: () = assert!(std::mem::size_of::<GpuEnvironment>() == 112);
