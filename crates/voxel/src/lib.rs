//! # The Vantablock Voxel Core
//!
//! A bevy-free library holding the engine's geometry contracts. Everything here runs on the CPU
//! and doubles as the reference implementation the GPU shaders are validated against.
//!
//! The crate deliberately separates four concepts (see the architecture spec, section 2.1):
//!
//! 1. **Canonical assets** ([`assembly`], [`module`]): a logical asset is a palette, a set of
//!    canonical voxel modules, and an articulated assembly of pose groups. This is the single
//!    source of truth and is what `.vanta` files ([`format`]) store.
//! 2. **Pose groups** ([`assembly::PoseGroupTemplate`], [`pose`]): each rigidly articulated
//!    group has a parent-relative transform, a motion domain, and its own immutable voxel root.
//! 3. **Spatial search** ([`bvh`], [`hierarchy`]): scene BVHs, template-shared object mini-BVHs,
//!    and pose-group-local voxel hierarchies are derived and fully regenerable.
//! 4. **Resolved runtime identity** ([`scene`]): a world instance plus one of its groups becomes a
//!    resolved pose frame, which is the identity visibility reports.
//!
//! Derived representations (hierarchies, LOD summaries, bounds, BVHs) may change freely; they must
//! never change which canonical geometry is visible.

pub mod arena;
pub mod assembly;
pub mod bvh;
pub mod format;
pub mod geometry;
pub mod handle;
pub mod hierarchy;
pub mod lod;
pub mod mask;
pub mod math;
pub mod module;
pub mod node;
pub mod pose;
pub mod reference;
pub mod residency;
pub mod scene;

pub use arena::{ArenaError, ArenaKind, Attributes, Handle, Topology, WordArena};
pub use assembly::{CanonicalAssembly, Material, Motion, PoseGroupTemplate};
pub use bvh::{BVH_EMPTY, BVH_LEAF_BIT, Bvh4, Bvh4Node};
pub use geometry::{GeometryArenas, GeometrySource};
pub use handle::ArenaHandle;
pub use hierarchy::{
    HierarchyKind, HitFace, VoxelHit, VoxelRoot, octree::OctreeRoot, tree64::Tree64Root,
};
pub use mask::Mask64;
pub use math::{Aabb, Ray};
pub use module::{MaterialId, VoxelModule};
pub use node::Node64;
pub use pose::{PoseDriver, PoseState};
