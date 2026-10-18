//! A 4-wide bounding volume hierarchy over conservative boxes.
//!
//! The same structure serves the scene BVH (leaves are instances or flattened pose frames) and
//! the template-shared object mini-BVH (leaves are pose-group slots). Nodes store their four
//! children's boxes in SoA order so the GPU tests all four with vector math, and the node layout
//! is shared bit-for-bit with `assets/client/shader/voxel/lib/bvh.wesl`.

use crate::math::{Aabb, Ray};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Set on a child reference that names a primitive rather than a node.
pub const BVH_LEAF_BIT: u32 = 0x8000_0000;
/// An unused child slot.
pub const BVH_EMPTY: u32 = u32::MAX;
/// Traversal stack depth. A median-split BVH4 over `n` primitives has depth `~log4(n)` and pushes
/// at most 3 siblings per level, so this covers far more primitives than the 20-bit frame IDs.
pub const BVH_STACK_SIZE: usize = 48;

const EMPTY_MIN: f32 = 1e30;

/// One BVH4 node: 128 bytes, 16 byte aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Bvh4Node {
    pub min_x: [f32; 4],
    pub min_y: [f32; 4],
    pub min_z: [f32; 4],
    pub max_x: [f32; 4],
    pub max_y: [f32; 4],
    pub max_z: [f32; 4],
    /// Node index, `BVH_LEAF_BIT | primitive`, or `BVH_EMPTY`.
    pub children: [u32; 4],
    pub _pad: [u32; 4],
}

const _: () = assert!(std::mem::size_of::<Bvh4Node>() == 128);

impl Bvh4Node {
    /// Unused slots hold an inverted finite box: WGSL may assume values are never infinite.
    fn empty() -> Self {
        Self {
            min_x: [EMPTY_MIN; 4],
            min_y: [EMPTY_MIN; 4],
            min_z: [EMPTY_MIN; 4],
            max_x: [-EMPTY_MIN; 4],
            max_y: [-EMPTY_MIN; 4],
            max_z: [-EMPTY_MIN; 4],
            children: [BVH_EMPTY; 4],
            _pad: [0; 4],
        }
    }

    pub fn child_bounds(&self, i: usize) -> Aabb {
        Aabb::new(
            Vec3::new(self.min_x[i], self.min_y[i], self.min_z[i]),
            Vec3::new(self.max_x[i], self.max_y[i], self.max_z[i]),
        )
    }

    fn set_child(&mut self, i: usize, child: u32, bounds: Aabb) {
        self.children[i] = child;
        self.min_x[i] = bounds.min.x;
        self.min_y[i] = bounds.min.y;
        self.min_z[i] = bounds.min.z;
        self.max_x[i] = bounds.max.x;
        self.max_y[i] = bounds.max.y;
        self.max_z[i] = bounds.max.z;
    }
}

/// Counters from one traversal, used for false-positive diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BvhTraversalStats {
    pub nodes_visited: u32,
    pub leaves_visited: u32,
}

/// A BVH4. `nodes[0]` is the root when the tree is non-empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bvh4 {
    pub nodes: Vec<Bvh4Node>,
}

impl Bvh4 {
    /// Builds a BVH whose leaf `i` holds `primitives[i]`. Empty boxes are skipped.
    pub fn build(primitives: &[Aabb]) -> Self {
        let mut indices: Vec<u32> = (0..primitives.len() as u32)
            .filter(|&i| !primitives[i as usize].is_empty())
            .collect();
        let mut bvh = Self::default();
        if !indices.is_empty() {
            bvh.build_node(primitives, &mut indices);
        }
        bvh
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The bound of everything in the tree.
    pub fn bounds(&self) -> Aabb {
        self.nodes.first().map_or(Aabb::EMPTY, |root| {
            (0..4).fold(Aabb::EMPTY, |acc, i| acc.union(&root.child_bounds(i)))
        })
    }

    fn build_node(&mut self, primitives: &[Aabb], indices: &mut [u32]) -> u32 {
        let node_index = self.nodes.len() as u32;
        self.nodes.push(Bvh4Node::empty());

        let mut node = Bvh4Node::empty();
        for (slot, group) in split_four(primitives, indices).into_iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let bounds = union_of(primitives, group);
            let child = if group.len() == 1 {
                BVH_LEAF_BIT | group[0]
            } else {
                self.build_node(primitives, group)
            };
            node.set_child(slot, child, bounds);
        }
        self.nodes[node_index as usize] = node;
        node_index
    }

    /// Recomputes every node's child boxes from new primitive bounds, keeping the topology.
    /// Quality degrades as primitives move; rebuild when candidate counts grow.
    pub fn refit(&mut self, primitives: &[Aabb]) {
        if !self.nodes.is_empty() {
            self.refit_node(0, primitives);
        }
    }

    fn refit_node(&mut self, index: u32, primitives: &[Aabb]) -> Aabb {
        let mut total = Aabb::EMPTY;
        for i in 0..4 {
            let child = self.nodes[index as usize].children[i];
            let bounds = if child == BVH_EMPTY {
                continue;
            } else if child & BVH_LEAF_BIT != 0 {
                primitives[(child & !BVH_LEAF_BIT) as usize]
            } else {
                self.refit_node(child, primitives)
            };
            self.nodes[index as usize].set_child(i, child, bounds);
            total = total.union(&bounds);
        }
        total
    }

    /// Visits leaves whose boxes the ray enters within `[t_min, t_max]`, nearest box first.
    ///
    /// `visit(primitive, t_enter, t_exit)` may return a closer `t_max` (a hit), which prunes
    /// every box entered beyond it. This is `query_scene_candidates` in the spec's contract.
    pub fn traverse(
        &self,
        ray: &Ray,
        t_min: f32,
        mut t_max: f32,
        mut visit: impl FnMut(u32, f32, f32) -> Option<f32>,
    ) -> BvhTraversalStats {
        let mut stats = BvhTraversalStats::default();
        if self.nodes.is_empty() {
            return stats;
        }

        let mut stack = [(0u32, 0f32); BVH_STACK_SIZE];
        let mut len = 1;
        stack[0] = (0, t_min);

        while len > 0 {
            len -= 1;
            let (child_ref, t_enter) = stack[len];
            if t_enter > t_max {
                continue;
            }
            if child_ref & BVH_LEAF_BIT != 0 {
                continue;
            }

            stats.nodes_visited += 1;
            let node = &self.nodes[child_ref as usize];
            let mut hits = [(0u32, 0f32, 0f32); 4];
            let mut count = 0;
            for i in 0..4 {
                let child = node.children[i];
                if child == BVH_EMPTY {
                    continue;
                }
                let (t0, t1) = node.child_bounds(i).ray_interval(ray);
                let t0 = t0.max(t_min);
                if t0 <= t1.min(t_max) {
                    hits[count] = (child, t0, t1);
                    count += 1;
                }
            }
            hits[..count].sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

            // leaves are visited nearest-first right away; nodes are pushed so the nearest pops
            // first
            for &(child, t0, t1) in &hits[..count] {
                if child & BVH_LEAF_BIT != 0 && t0 <= t_max {
                    stats.leaves_visited += 1;
                    if let Some(t) = visit(child & !BVH_LEAF_BIT, t0, t1) {
                        t_max = t_max.min(t);
                    }
                }
            }
            for &(child, t0, _) in hits[..count].iter().rev() {
                if child & BVH_LEAF_BIT == 0 {
                    assert!(len < BVH_STACK_SIZE, "BVH traversal stack overflow");
                    stack[len] = (child, t0);
                    len += 1;
                }
            }
        }
        stats
    }
}

fn union_of(primitives: &[Aabb], indices: &[u32]) -> Aabb {
    indices
        .iter()
        .fold(Aabb::EMPTY, |acc, &i| acc.union(&primitives[i as usize]))
}

/// Splits primitives into up to four spatially coherent groups using two levels of median
/// splits along the axis of greatest centroid spread.
fn split_four<'a>(primitives: &[Aabb], indices: &'a mut [u32]) -> [&'a mut [u32]; 4] {
    if indices.len() <= 4 {
        let n = indices.len();
        let (a, rest) = indices.split_at_mut(n.min(1));
        let (b, rest) = rest.split_at_mut(n.saturating_sub(1).min(1));
        let (c, d) = rest.split_at_mut(n.saturating_sub(2).min(1));
        return [a, b, c, d];
    }
    let (left, right) = median_split(primitives, indices);
    let (a, b) = median_split(primitives, left);
    let (c, d) = median_split(primitives, right);
    [a, b, c, d]
}

fn median_split<'a>(primitives: &[Aabb], indices: &'a mut [u32]) -> (&'a mut [u32], &'a mut [u32]) {
    let centroids = indices.iter().fold(Aabb::EMPTY, |acc, &i| {
        let c = primitives[i as usize].center();
        acc.union(&Aabb::new(c, c))
    });
    let axis = centroids.extent().max_position();
    let mid = indices.len() / 2;
    indices.select_nth_unstable_by(mid, |&a, &b| {
        primitives[a as usize].center()[axis].total_cmp(&primitives[b as usize].center()[axis])
    });
    indices.split_at_mut(mid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_boxes(n: u32) -> Vec<Aabb> {
        (0..n)
            .map(|i| {
                let p = Vec3::new(
                    (i % 10) as f32 * 3.0,
                    (i / 10 % 10) as f32 * 3.0,
                    (i / 100) as f32 * 3.0,
                );
                Aabb::new(p, p + Vec3::ONE)
            })
            .collect()
    }

    /// Every primitive a ray's box test passes must be visited (no missed candidates).
    #[test]
    fn traversal_finds_every_intersected_primitive() {
        let boxes = grid_boxes(300);
        let bvh = Bvh4::build(&boxes);
        let mut state = 1u32;
        for _ in 0..200 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let o = Vec3::new(-5.0, (state % 31) as f32, ((state >> 8) % 11) as f32);
            let d = Vec3::new(1.0, ((state >> 16) % 7) as f32 * 0.05 - 0.15, 0.03).normalize();
            let ray = Ray::new(o, d);

            let mut visited = Vec::new();
            bvh.traverse(&ray, 0.0, f32::MAX, |p, _, _| {
                visited.push(p);
                None
            });
            for (i, b) in boxes.iter().enumerate() {
                let (t0, t1) = b.ray_interval(&ray);
                if t0.max(0.0) <= t1 {
                    assert!(visited.contains(&(i as u32)), "missed primitive {i}");
                }
            }
        }
    }

    #[test]
    fn closer_hits_prune_farther_boxes() {
        let boxes: Vec<Aabb> = (0..8)
            .map(|i| {
                Aabb::new(
                    Vec3::new(i as f32 * 4.0, 0.0, 0.0),
                    Vec3::new(i as f32 * 4.0 + 1.0, 1.0, 1.0),
                )
            })
            .collect();
        let bvh = Bvh4::build(&boxes);
        let ray = Ray::new(Vec3::new(-1.0, 0.5, 0.5), Vec3::X);
        let mut visited = Vec::new();
        bvh.traverse(&ray, 0.0, f32::MAX, |p, t0, _| {
            visited.push(p);
            Some(t0)
        });
        assert_eq!(visited, vec![0], "a hit in the first box prunes the rest");
    }

    #[test]
    fn refit_tracks_moved_primitives() {
        let mut boxes = grid_boxes(50);
        let mut bvh = Bvh4::build(&boxes);
        for b in &mut boxes {
            b.min += Vec3::splat(100.0);
            b.max += Vec3::splat(100.0);
        }
        bvh.refit(&boxes);
        assert!(bvh.bounds().min.x >= 100.0);
        let ray = Ray::new(Vec3::new(90.0, 100.5, 100.5), Vec3::X);
        let mut hit = false;
        bvh.traverse(&ray, 0.0, f32::MAX, |_, _, _| {
            hit = true;
            None
        });
        assert!(hit);
    }

    #[test]
    fn single_and_empty_trees() {
        assert!(Bvh4::build(&[]).is_empty());
        let one = Bvh4::build(&[Aabb::new(Vec3::ZERO, Vec3::ONE)]);
        assert_eq!(one.nodes.len(), 1);
        assert_eq!(one.nodes[0].children[0], BVH_LEAF_BIT);
    }
}
