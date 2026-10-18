//! Pose-group-local voxel hierarchies.
//!
//! Each rigid pose group's canonical module is baked into an immutable local hierarchy that
//! rejects empty microvoxel space. Two hierarchies bake from the same [`VoxelModule`] and are
//! held to the same reference ([`crate::reference`]):
//!
//! - [`tree64`]: the primary candidate, a 64-way tree of `4 x 4 x 4` nodes.
//! - [`octree`]: the spec's comparison baseline, an 8-way octree ending in `8 x 8 x 8` bricks.
//!
//! Both traverse with the same hierarchical DDA rules (entry cells from plane distances,
//! boundaries recomputed from integer cells, lowest-axis tie breaks), so a difference in cost
//! between them is the hierarchy's, not the traversal's.
//!
//! Every hierarchy must report exact integer voxel identity for its hits, never a cell recovered
//! by flooring a floating point hit position.

pub mod octree;
pub mod tree64;

use crate::{
    geometry::GeometrySource,
    handle::ArenaHandle,
    math::{Aabb, Ray},
    module::{MaterialId, VoxelModule},
};
use glam::{IVec3, UVec3, Vec3};

/// Traversal gives up (reporting a miss) after this many DDA steps, so a bad ray can never hang
/// the GPU. Diagnostics count rays that hit the cap.
pub const MAX_TRAVERSAL_STEPS: u32 = 4096;

/// Attribute word layout: material in the low 16 bits, the rest reserved for per-voxel data.
pub const ATTRIBUTE_MATERIAL_MASK: u32 = 0xFFFF;

/// Which local hierarchy a voxel root was baked as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HierarchyKind {
    #[default]
    Tree64,
    Octree,
}

impl HierarchyKind {
    pub const ALL: [Self; 2] = [Self::Tree64, Self::Octree];
}

/// A baked voxel root of either hierarchy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoxelRoot {
    Tree64(tree64::Tree64Root),
    Octree(octree::OctreeRoot),
}

impl VoxelRoot {
    pub fn kind(&self) -> HierarchyKind {
        match self {
            Self::Tree64(_) => HierarchyKind::Tree64,
            Self::Octree(_) => HierarchyKind::Octree,
        }
    }

    /// The root node's logical topology handle.
    pub fn node(&self) -> ArenaHandle {
        match self {
            Self::Tree64(r) => r.node,
            Self::Octree(r) => r.node,
        }
    }

    /// Hierarchy levels below the root (64-tree levels, or octree levels above the bricks).
    pub fn levels(&self) -> u8 {
        match self {
            Self::Tree64(r) => r.levels,
            Self::Octree(r) => r.depth,
        }
    }

    pub fn single_page(&self) -> bool {
        match self {
            Self::Tree64(r) => r.single_page,
            Self::Octree(r) => r.single_page,
        }
    }

    pub fn bounds(&self) -> Aabb {
        match self {
            Self::Tree64(r) => r.bounds,
            Self::Octree(r) => r.bounds,
        }
    }

    /// Traces `ray` (in module voxel space) within `[t_min, t_max]`, also returning the DDA steps
    /// taken, including on misses.
    pub fn trace_counted(
        &self,
        geometry: &impl GeometrySource,
        ray: &Ray,
        t_min: f32,
        t_max: f32,
    ) -> (Option<VoxelHit>, u32) {
        match self {
            Self::Tree64(r) => tree64::trace_counted(geometry, r, ray, t_min, t_max),
            Self::Octree(r) => octree::trace_counted(geometry, r, ray, t_min, t_max),
        }
    }

    pub fn trace(
        &self,
        geometry: &impl GeometrySource,
        ray: &Ray,
        t_min: f32,
        t_max: f32,
    ) -> Option<VoxelHit> {
        self.trace_counted(geometry, ray, t_min, t_max).0
    }
}

/// The face a ray entered a voxel through, or `Inside` when it started inside the voxel.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitFace {
    NegX = 0,
    PosX = 1,
    NegY = 2,
    PosY = 3,
    NegZ = 4,
    PosZ = 5,
    Inside = 6,
}

impl HitFace {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::NegX,
            1 => Self::PosX,
            2 => Self::NegY,
            3 => Self::PosY,
            4 => Self::NegZ,
            5 => Self::PosZ,
            _ => Self::Inside,
        }
    }

    /// The outward normal of the face in voxel space.
    pub fn normal(self) -> Vec3 {
        match self {
            Self::NegX => Vec3::NEG_X,
            Self::PosX => Vec3::X,
            Self::NegY => Vec3::NEG_Y,
            Self::PosY => Vec3::Y,
            Self::NegZ => Vec3::NEG_Z,
            Self::PosZ => Vec3::Z,
            Self::Inside => Vec3::ZERO,
        }
    }
}

/// An opaque voxel hit with exact integer identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelHit {
    /// Distance along the traced ray (equal to world distance, see [`crate::math`]).
    pub t: f32,
    /// The hit voxel in module space, taken from traversal state rather than the hit position.
    pub cell: UVec3,
    /// The voxel's 6-bit index within its aligned `4 x 4 x 4` block, which is the 64-tree's
    /// terminal node. Every hierarchy reports the same index for the same cell.
    pub terminal_index: u32,
    pub face: HitFace,
    /// Logical handle of the hit voxel's attribute word.
    pub attribute: ArenaHandle,
    pub material: MaterialId,
    /// DDA steps taken, for diagnostics.
    pub steps: u32,
}

/// Bakes `module` as `kind` into `arenas`.
pub fn bake(
    arenas: &mut crate::geometry::GeometryArenas,
    module: &VoxelModule,
    kind: HierarchyKind,
) -> Result<VoxelRoot, crate::arena::ArenaError> {
    Ok(match kind {
        HierarchyKind::Tree64 => VoxelRoot::Tree64(tree64::bake(arenas, module)?.0),
        HierarchyKind::Octree => VoxelRoot::Octree(octree::bake(arenas, module)?.0),
    })
}

/// Counters from one bake.
#[derive(Clone, Copy, Debug, Default)]
pub struct BakeStats {
    /// Every node, terminal nodes and bricks included.
    pub nodes: u64,
    /// Nodes whose children are voxels: 64-tree terminal nodes, or octree bricks.
    pub terminal_nodes: u64,
    pub voxels: u64,
}

/// The logical pages a bake's topology blocks landed in.
#[derive(Default)]
pub(crate) struct PageSpan {
    min: Option<u32>,
    max: u32,
}

impl PageSpan {
    pub(crate) fn include(&mut self, handle: ArenaHandle) {
        let page = handle.page();
        self.min = Some(self.min.map_or(page, |m| m.min(page)));
        self.max = self.max.max(page);
    }

    pub(crate) fn is_single(&self, page: u32) -> bool {
        self.min == Some(page) && self.max == page
    }
}

// INFO: ------------------------------
//         shared DDA rules
// ------------------------------------

/// The step direction of a ray along each axis.
pub(crate) fn step_of(ray: &Ray) -> IVec3 {
    IVec3::new(
        if ray.dir.x < 0.0 { -1 } else { 1 },
        if ray.dir.y < 0.0 { -1 } else { 1 },
        if ray.dir.z < 0.0 { -1 } else { 1 },
    )
}

/// The child cell of a node with `cells` cells per axis that the ray occupies at `t`.
///
/// Cells are chosen by counting the node's internal planes the ray has crossed by `t`, using the
/// same `(plane - origin) * inv_dir` distances as the slab test. Reconstructing the entry point
/// and flooring it instead breaks for rays within an ulp of a plane: the rounded position lands
/// in the neighboring cell, and a ray running along that plane then misses everything beside it.
pub(crate) fn entry_cell(
    origin: IVec3,
    size: i32,
    cells: i32,
    ray: &Ray,
    step: IVec3,
    t: f32,
) -> IVec3 {
    let mut cell = IVec3::ZERO;
    for axis in 0..3 {
        let mut crossed = 0;
        for j in 1..cells {
            // planes in the order the ray meets them
            let plane = if step[axis] > 0 { j } else { cells - j };
            let t_plane =
                ((origin[axis] + plane * size) as f32 - ray.origin[axis]) * ray.inv_dir[axis];
            if t_plane > t {
                break;
            }
            crossed += 1;
        }
        cell[axis] = if step[axis] > 0 {
            crossed
        } else {
            cells - 1 - crossed
        };
    }
    cell
}

/// The axis of the nearest boundary; ties pick the lowest axis, matching the shaders.
pub(crate) fn nearest_axis(t: Vec3) -> usize {
    if t.x <= t.y && t.x <= t.z {
        0
    } else if t.y <= t.z {
        1
    } else {
        2
    }
}

/// The face a ray entered the cell `[lo, lo + size)` through.
pub(crate) fn entry_face(
    lo: IVec3,
    size: i32,
    ray: &Ray,
    step: IVec3,
    te: f32,
    t_start: f32,
) -> HitFace {
    if te < t_start {
        return HitFace::Inside;
    }
    let t0 = (lo.as_vec3() - ray.origin) * ray.inv_dir;
    let t1 = ((lo + IVec3::splat(size)).as_vec3() - ray.origin) * ray.inv_dir;
    let near = t0.min(t1);
    let axis = if near.x >= near.y && near.x >= near.z {
        0
    } else if near.y >= near.z {
        1
    } else {
        2
    };
    // stepping up enters through the min face, whose outward normal points down the axis
    HitFace::from_code(2 * axis + (step[axis as usize] < 0) as u32)
}
