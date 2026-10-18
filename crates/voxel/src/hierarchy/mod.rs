//! Pose-group-local voxel hierarchies.
//!
//! Each rigid pose group's canonical module is baked into an immutable local hierarchy that
//! rejects empty microvoxel space. The 64-tree is the primary candidate; the spec's comparison
//! baselines (an 8-way sparse octree with bricks and a sparse brick grid) belong here as sibling
//! modules baking from the same [`crate::module::VoxelModule`].
//!
//! Every hierarchy must report exact integer voxel identity for its hits, never a cell recovered
//! by flooring a floating point hit position.

pub mod tree64;
