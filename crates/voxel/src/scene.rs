//! Scene-level search: world instances, resolved pose frames, and nearest-hit queries.
//!
//! ```text
//! world ray
//!   -> scene BVH4 (instances, or flattened pose frames)
//!     -> template-shared mini-BVH (group slots, in asset root space)
//!       -> resolved pose frame (instance + group)
//!         -> group-local voxel hierarchy
//! ```
//!
//! A [`SceneSnapshot`] is rebuilt from authoritative instance state whenever poses change. It is
//! hierarchy-agnostic: callers pass the per-frame intersection, so every local voxel backend
//! plugs into the same nearest-hit ordering. The GPU mirrors [`SceneSnapshot::intersect`] in
//! `assets/client/shader/voxel/lib/scene.wesl`.

use crate::{
    assembly::{CanonicalAssembly, PoseGroupTemplate, group_to_root},
    bvh::Bvh4,
    hierarchy::tree64::VoxelHit,
    math::{Aabb, Ray},
    pose::PoseState,
};
use glam::Affine3A;
use std::collections::HashMap;

/// Frame IDs are 20 bits in the visibility buffer.
pub const MAX_POSE_FRAMES: usize = 1 << 20;
/// A template group slot with no resolved frame.
pub const INVALID_FRAME: u32 = u32::MAX;

/// How pose groups are organized for spatial search. Both modes must find identical hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SceneAcceleration {
    /// Option B: scene leaves are instances; each articulated instance is searched with its
    /// asset's template-shared mini-BVH.
    #[default]
    TemplateMiniBvh,
    /// Option A: every resolved pose frame is its own scene leaf.
    FlattenedGroups,
}

/// Immutable, derived search data for one asset, shared by all of its instances.
#[derive(Clone, Debug)]
pub struct AssetTemplate {
    pub groups: Vec<PoseGroupTemplate>,
    /// Occupied bounds of each group's module, in group-local voxels.
    pub group_bounds: Vec<Aabb>,
    /// Conservative per-group bounds over every legal pose, in asset root space.
    pub envelopes: Vec<Aabb>,
    pub root_envelope: Aabb,
    /// Built over `envelopes` once per asset. `None` for single-group assets, which trace their
    /// one root directly.
    pub mini_bvh: Option<Bvh4>,
}

impl AssetTemplate {
    pub fn new(assembly: &CanonicalAssembly) -> Self {
        let envelopes = assembly.motion_envelopes();
        Self {
            groups: assembly.groups.clone(),
            group_bounds: assembly
                .groups
                .iter()
                .map(|g| assembly.module_bounds(g.module))
                .collect(),
            root_envelope: envelopes.iter().fold(Aabb::EMPTY, |a, b| a.union(b)),
            mini_bvh: assembly.needs_mini_bvh().then(|| Bvh4::build(&envelopes)),
            envelopes,
        }
    }

    pub fn group_count(&self) -> u32 {
        self.groups.len() as u32
    }
}

/// An authoritative world instance, as replicated from the server.
#[derive(Clone, Debug)]
pub struct InstanceDesc {
    /// Stable identity across snapshots, used to find last frame's transforms for history.
    pub key: u64,
    pub asset: u32,
    pub root_to_world: Affine3A,
    pub pose: PoseState,
}

/// The runtime identity visibility reports: one world instance's one articulated group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedPoseFrame {
    pub instance: u32,
    pub asset: u32,
    pub group: u32,
    pub group_to_world: Affine3A,
    pub world_to_group: Affine3A,
    /// Last snapshot's transform, for motion vectors (equal to `group_to_world` when new).
    pub prev_group_to_world: Affine3A,
    /// Tight bound of the group's current pose, in world space.
    pub world_bounds: Aabb,
}

/// Per-instance search data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceRecord {
    pub asset: u32,
    pub world_to_root: Affine3A,
    pub world_bounds: Aabb,
    /// Start of this instance's slots in [`SceneSnapshot::slot_frames`].
    pub slot_base: u32,
    pub group_count: u32,
    /// `false` when the pose left its baked envelopes, so the template mini-BVH can't be trusted
    /// and every slot is tested against its current bound instead.
    pub uses_template: bool,
}

/// Counters for one scene query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneTraceStats {
    pub scene_nodes: u32,
    pub instance_candidates: u32,
    pub group_candidates: u32,
    pub voxel_traces: u32,
}

/// The nearest hit of a scene query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneHit {
    pub frame: u32,
    pub voxel: VoxelHit,
}

/// Everything needed to answer ray queries for one moment in time. See the [module docs](self).
#[derive(Clone, Debug, Default)]
pub struct SceneSnapshot {
    pub mode: SceneAcceleration,
    pub frames: Vec<ResolvedPoseFrame>,
    pub instances: Vec<InstanceRecord>,
    /// Instance-owned indirection from template group slot to resolved frame ID. Instances never
    /// assume their frames are contiguous, so missing or reordered groups stay correct.
    pub slot_frames: Vec<u32>,
    /// Scene leaves are instance indices or frame indices, depending on `mode`.
    pub bvh: Bvh4,
    /// Frames that did not fit the 20-bit frame ID space and are not rendered.
    pub overflowed_frames: usize,
    history: HashMap<(u64, u32), Affine3A>,
}

impl SceneSnapshot {
    pub fn build(
        templates: &[AssetTemplate],
        instances: &[InstanceDesc],
        mode: SceneAcceleration,
        previous: Option<&SceneSnapshot>,
    ) -> Self {
        let mut snapshot = Self {
            mode,
            ..Default::default()
        };

        for (instance_index, desc) in instances.iter().enumerate() {
            let Some(template) = templates.get(desc.asset as usize) else {
                continue;
            };
            let to_root = group_to_root(&template.groups, &desc.pose);
            let slot_base = snapshot.slot_frames.len() as u32;
            let mut current_bounds = Aabb::EMPTY;

            for (group, root_from_group) in to_root.iter().enumerate() {
                if snapshot.frames.len() >= MAX_POSE_FRAMES {
                    snapshot.overflowed_frames += 1;
                    snapshot.slot_frames.push(INVALID_FRAME);
                    continue;
                }
                let group_to_world = desc.root_to_world * *root_from_group;
                let world_bounds = template.group_bounds[group].transformed(&group_to_world);
                current_bounds = current_bounds.union(&world_bounds);

                let history_key = (desc.key, group as u32);
                let prev_group_to_world = previous
                    .and_then(|p| p.history.get(&history_key).copied())
                    .unwrap_or(group_to_world);
                snapshot.history.insert(history_key, group_to_world);

                snapshot.slot_frames.push(snapshot.frames.len() as u32);
                snapshot.frames.push(ResolvedPoseFrame {
                    instance: instance_index as u32,
                    asset: desc.asset,
                    group: group as u32,
                    group_to_world,
                    world_to_group: group_to_world.inverse(),
                    prev_group_to_world,
                    world_bounds,
                });
            }

            let uses_template = template.mini_bvh.is_some()
                && desc.pose.angles.len() == template.groups.len()
                && template
                    .groups
                    .iter()
                    .zip(&desc.pose.angles)
                    .all(|(g, &a)| g.motion.allows(a));
            let world_bounds = if uses_template {
                template.root_envelope.transformed(&desc.root_to_world)
            } else {
                current_bounds
            };

            snapshot.instances.push(InstanceRecord {
                asset: desc.asset,
                world_to_root: desc.root_to_world.inverse(),
                world_bounds,
                slot_base,
                group_count: template.group_count(),
                uses_template,
            });
        }

        let leaf_bounds: Vec<Aabb> = match mode {
            SceneAcceleration::TemplateMiniBvh => {
                snapshot.instances.iter().map(|i| i.world_bounds).collect()
            }
            SceneAcceleration::FlattenedGroups => {
                snapshot.frames.iter().map(|f| f.world_bounds).collect()
            }
        };
        snapshot.bvh = Bvh4::build(&leaf_bounds);
        snapshot
    }

    /// Finds the nearest hit along a world ray.
    ///
    /// `trace_frame(frame, local_ray, t_min, t_max)` intersects one frame's local hierarchy with
    /// the ray already in group-local space; `t` stays a world distance throughout.
    pub fn intersect(
        &self,
        templates: &[AssetTemplate],
        ray: &Ray,
        t_min: f32,
        t_max: f32,
        mut trace_frame: impl FnMut(&ResolvedPoseFrame, &Ray, f32, f32) -> Option<VoxelHit>,
    ) -> (Option<SceneHit>, SceneTraceStats) {
        let mut best: Option<SceneHit> = None;
        let mut stats = SceneTraceStats::default();

        let mut test_frame = |frame_id: u32, t_best: f32, stats: &mut SceneTraceStats| {
            let frame = &self.frames[frame_id as usize];
            stats.voxel_traces += 1;
            let local = ray.transformed(&frame.world_to_group);
            trace_frame(frame, &local, t_min, t_best).map(|voxel| SceneHit {
                frame: frame_id,
                voxel,
            })
        };

        let bvh_stats = self.bvh.traverse(ray, t_min, t_max, |leaf, _, _| {
            let t_best = best.map_or(t_max, |b| b.voxel.t);
            let found = match self.mode {
                SceneAcceleration::FlattenedGroups => {
                    stats.group_candidates += 1;
                    test_frame(leaf, t_best, &mut stats)
                }
                SceneAcceleration::TemplateMiniBvh => {
                    stats.instance_candidates += 1;
                    self.intersect_instance(
                        templates,
                        leaf,
                        ray,
                        t_min,
                        t_best,
                        &mut stats,
                        &mut test_frame,
                    )
                }
            };
            match found {
                Some(hit) if best.is_none_or(|b| hit.voxel.t < b.voxel.t) => {
                    best = Some(hit);
                    Some(hit.voxel.t)
                }
                _ => None,
            }
        });
        stats.scene_nodes = bvh_stats.nodes_visited;
        (best, stats)
    }

    /// Searches one instance's groups (`intersect_instance` in the spec's contract).
    #[allow(clippy::too_many_arguments)]
    fn intersect_instance(
        &self,
        templates: &[AssetTemplate],
        instance_index: u32,
        ray: &Ray,
        t_min: f32,
        t_max: f32,
        stats: &mut SceneTraceStats,
        test_frame: &mut impl FnMut(u32, f32, &mut SceneTraceStats) -> Option<SceneHit>,
    ) -> Option<SceneHit> {
        let instance = &self.instances[instance_index as usize];
        let mut best: Option<SceneHit> = None;

        let mut try_slot = |slot: u32, stats: &mut SceneTraceStats, best: &mut Option<SceneHit>| {
            let frame_id = self.slot_frames[(instance.slot_base + slot) as usize];
            if frame_id == INVALID_FRAME {
                return None;
            }
            // per-instance refinement: the frame's tight current bound
            let t_best = best.map_or(t_max, |b| b.voxel.t);
            let (t0, t1) = self.frames[frame_id as usize]
                .world_bounds
                .ray_interval(ray);
            if t0.max(t_min) > t1.min(t_best) {
                return None;
            }
            stats.group_candidates += 1;
            let hit = test_frame(frame_id, t_best, stats)?;
            if best.is_none_or(|b| hit.voxel.t < b.voxel.t) {
                *best = Some(hit);
                return Some(hit.voxel.t);
            }
            None
        };

        let template = &templates[instance.asset as usize];
        match (&template.mini_bvh, instance.uses_template) {
            (Some(mini_bvh), true) => {
                let root_ray = ray.transformed(&instance.world_to_root);
                mini_bvh.traverse(&root_ray, t_min, t_max, |slot, _, _| {
                    try_slot(slot, stats, &mut best)
                });
            }
            // single-root assets and out-of-envelope poses test every slot directly
            _ => {
                for slot in 0..instance.group_count {
                    try_slot(slot, stats, &mut best);
                }
            }
        }
        best
    }
}
