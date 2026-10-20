//! GPU-facing data layouts, mirrored field for field in
//! `assets/client/shader/voxel/lib/types.wesl`. Every struct here is `Pod` and its size is
//! asserted, so a layout drift fails to compile instead of corrupting the GPU's view.

use bevy::math::Affine3A;
use bytemuck::{Pod, Zeroable};
use voxel::{HierarchyKind, VoxelRoot, handle::MAX_PAGES, scene::ResolvedPoseFrame};

/// `PoseFrame::flags`: every topology block of the frame's root lives in one page.
pub const FRAME_SINGLE_PAGE: u32 = 1;
/// `PoseFrame::flags`: the frame's root is an octree rather than a 64-tree.
pub const FRAME_OCTREE: u32 = 2;
/// `InstanceRecord::flags`: search this instance with its template mini-BVH.
pub const INSTANCE_USES_TEMPLATE: u32 = 1;
/// `InstanceRecord::flags`: search this instance with its own refit mini-BVH, whose nodes follow
/// the scene BVH's in the scene node buffer.
pub const INSTANCE_OWN_BVH: u32 = 2;
/// `InstanceRecord::mini_bvh_root` for single-root assets.
pub const NO_MINI_BVH: u32 = u32::MAX;
/// Visibility buffer value for pixels that hit nothing.
pub const VISIBILITY_MISS: u32 = u32::MAX;

/// `SceneUniform::mode` values.
pub const MODE_TEMPLATE_MINI_BVH: u32 = 0;
pub const MODE_FLATTENED_GROUPS: u32 = 1;
pub const MODE_INSTANCE_REFIT: u32 = 2;

/// `ViewUniform::flags`: the target is not HDR, so the shader tonemaps itself.
pub const VIEW_TONEMAP_IN_SHADER: u32 = 1;
/// `ViewUniform::flags`: footprint LOD filtering draws each traced pixel's opacity threshold from
/// a stable per-pixel hash (stochastic transmission) instead of `footprint_threshold`.
pub const VIEW_STOCHASTIC_FOOTPRINT: u32 = 2;

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
    /// Slab address of the root's page, for the single-page fast path.
    pub root_page_base: u32,
    pub levels: u32,
    pub flags: u32,
    pub palette_base: u32,
    pub instance: u32,
    /// The root's occupied bounds in group-local voxels (see [`root_bounds`]), which clip
    /// footprint-filtered hits.
    pub local_bounds_min: [u32; 3],
    pub pad0: u32,
    pub local_bounds_max: [u32; 3],
    pub pad1: u32,
}
const _: () = assert!(std::mem::size_of::<GpuPoseFrame>() == 224);

impl GpuPoseFrame {
    pub fn new(
        frame: &ResolvedPoseFrame,
        root: &VoxelRoot,
        root_page_base: u32,
        palette_base: u32,
    ) -> Self {
        let (local_bounds_min, local_bounds_max) = root_bounds(root);
        Self {
            world_to_local: affine_rows(&frame.world_to_group),
            local_to_world: affine_rows(&frame.group_to_world),
            prev_local_to_world: affine_rows(&frame.prev_group_to_world),
            bounds_min: frame.world_bounds.min.to_array(),
            root_node: root.node().raw(),
            bounds_max: frame.world_bounds.max.to_array(),
            root_page_base,
            levels: root.levels() as u32,
            flags: root_flags(root),
            palette_base,
            instance: frame.instance,
            local_bounds_min,
            pad0: 0,
            local_bounds_max,
            pad1: 0,
        }
    }
}

/// A root's occupied bounds in module voxels as integer `(min, max)` corners (`max` exclusive),
/// zero for an empty root. Footprint-filtered traversal clips LOD hits to them.
pub fn root_bounds(root: &VoxelRoot) -> ([u32; 3], [u32; 3]) {
    let bounds = root.bounds();
    if bounds.is_empty() {
        return ([0; 3], [0; 3]);
    }
    (
        bounds.min.as_uvec3().to_array(),
        bounds.max.as_uvec3().to_array(),
    )
}

/// The `PoseFrame::flags` describing how to trace `root`.
pub fn root_flags(root: &VoxelRoot) -> u32 {
    let page = if root.single_page() {
        FRAME_SINGLE_PAGE
    } else {
        0
    };
    let kind = match root.kind() {
        HierarchyKind::Tree64 => 0,
        HierarchyKind::Octree => FRAME_OCTREE,
    };
    page | kind
}

/// A world instance, the scene BVH leaf in template mini-BVH mode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuInstance {
    pub world_to_root: AffineRows,
    /// The instance's mini-BVH root: in the template node buffer, or in the scene node buffer
    /// with `INSTANCE_OWN_BVH`.
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

/// Logical page to slab address (slab in the top bits, word below), for topology and attribute
/// handles. Pages are packed four per `vec4<u32>` because uniform arrays have a 16 byte stride.
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
    /// Last frame's `clip_from_world`, for motion vectors (equal to the current one on a view's
    /// first frame).
    pub prev_clip_from_world: [f32; 16],
    pub camera_position: [f32; 3],
    pub debug_view: u32,
    /// Size of the traced ray grid (the visibility buffer), in pixels.
    pub viewport: [f32; 2],
    pub flags: u32,
    /// Ray grid pixels per view target pixel (the render scale).
    pub target_to_grid: f32,
    /// Footprint LOD filtering (the `footprint_lod` primary visibility variants): the opacity
    /// threshold, and the width of a pixel's ray cone in traced pixels.
    pub footprint_threshold: f32,
    pub footprint_scale: f32,
    /// The footprint transmission variants (`RenderSettings::footprint_transmission`): the opacity
    /// from which a filtered hit counts as opaque and nothing is traced behind it.
    pub transmission_opaque: f32,
    pub _pad: u32,
}
const _: () = assert!(std::mem::size_of::<GpuViewUniform>() == 240);

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
