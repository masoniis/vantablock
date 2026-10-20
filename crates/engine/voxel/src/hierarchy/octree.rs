//! The 8-way sparse octree with `8 x 8 x 8` voxel bricks: the spec's comparison hierarchy.
//!
//! Internal nodes split their cube in two along each axis. Nodes on the last internal level point
//! at bricks instead of nodes, each a dense 512-bit occupancy mask over `8^3` voxels. A root of
//! `depth` internal levels covers `8 * 2^depth` voxels per axis; modules smaller than 16 voxels
//! still get one internal level, so the root is always a node.
//!
//! Both record types live in the shared topology arena, typed by the root that consumes them:
//!
//! ```text
//! node  (2 words):  child_mask (bits 0..8) | BRICK_CHILDREN (bit 31), child_ref
//! brick (17 words): occupancy (16 words, voxel x + 8y + 64z), attribute block handle
//! ```
//!
//! Child `k = x + 2y + 4z` of a node sits at `child_ref + popcount(mask below k) * stride`, where
//! the stride is a node's or a brick's size. A brick's attribute block holds one word per occupied
//! voxel in occupancy order. Identical child blocks and attribute blocks are deduplicated exactly
//! as in the 64-tree.
//!
//! Traversal follows the shared DDA rules (see [`super`]) with two cells per axis on internal
//! levels and eight inside a brick, and mirrors `assets/client/shader/voxel/lib/octree.wesl`.
//! Octree nodes carry no LOD summaries; nothing reads them yet.

use super::{
    ATTRIBUTE_MATERIAL_MASK, BakeStats, MAX_TRAVERSAL_STEPS, PageSpan, VoxelHit, entry_cell,
    entry_face, nearest_axis, step_of,
};
use crate::{
    arena::ArenaError,
    geometry::{GeometryArenas, GeometrySource},
    handle::ArenaHandle,
    mask::cell_index,
    math::{Aabb, Ray, slab},
    module::{EMPTY, MaterialId, VoxelModule},
};
use glam::{IVec3, UVec3, Vec3};
use std::collections::HashMap;

/// Size of a node record in 32-bit words.
pub const NODE_WORDS: u32 = 2;
/// Voxels per brick edge.
pub const BRICK_EDGE: u32 = 8;
pub const BRICK_VOXELS: usize = 512;
/// Occupancy words per brick.
pub const BRICK_MASK_WORDS: u32 = 16;
/// Size of a brick record in 32-bit words: occupancy plus the attribute handle.
pub const BRICK_WORDS: u32 = BRICK_MASK_WORDS + 1;
/// Low 8 bits of a node's first word: one occupancy bit per octant.
pub const CHILD_MASK: u32 = 0xFF;
/// Bit 31 of a node's first word: set when the children are bricks.
pub const BRICK_CHILDREN: u32 = 0x8000_0000;
/// Internal levels of a `1024^3` module.
pub const MAX_DEPTH: u8 = 7;

/// A baked, immutable octree root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OctreeRoot {
    /// The root node record (a one-node block in the topology arena).
    pub node: ArenaHandle,
    /// Internal levels above the bricks.
    pub depth: u8,
    /// Whether every topology block reachable from the root lives in the root's logical page.
    pub single_page: bool,
    /// Occupied bounds in module voxel space.
    pub bounds: Aabb,
}

impl OctreeRoot {
    /// Voxels per axis covered by the root.
    pub fn extent(&self) -> u32 {
        BRICK_EDGE << self.depth
    }
}

/// Internal levels needed to cover a module of `extent` voxels per axis.
pub fn depth_for_extent(extent: u32) -> u8 {
    let side = extent.max(2 * BRICK_EDGE).next_power_of_two();
    (side.trailing_zeros() - BRICK_EDGE.trailing_zeros()) as u8
}

/// A voxel's bit within its brick.
#[inline]
pub const fn brick_index(x: u32, y: u32, z: u32) -> u32 {
    x + 8 * y + 64 * z
}

// INFO: --------------
//         bake
// --------------------

type BrickMaterials = Box<[MaterialId; BRICK_VOXELS]>;

/// Bakes `module` into `arenas`. See the [module docs](self).
pub fn bake(
    arenas: &mut GeometryArenas,
    module: &VoxelModule,
) -> Result<(OctreeRoot, BakeStats), ArenaError> {
    let depth = depth_for_extent(module.extent()) as u32;
    debug_assert!(depth <= MAX_DEPTH as u32);

    // regroup the module's 4^3 bricks into 8^3 bricks
    let mut grouped: HashMap<UVec3, BrickMaterials> = HashMap::new();
    for (&coord, brick) in module.bricks() {
        let materials = grouped
            .entry(coord / 2)
            .or_insert_with(|| Box::new([EMPTY; BRICK_VOXELS]));
        let offset = (coord % 2) * 4;
        for k in brick.occupancy().iter() {
            let p = offset + UVec3::new(k & 3, (k >> 2) & 3, k >> 4);
            materials[brick_index(p.x, p.y, p.z) as usize] = brick.materials[k as usize];
        }
    }
    let mut bricks: Vec<(u32, BrickMaterials)> = grouped
        .into_iter()
        .map(|(coord, materials)| (octant_key(coord, depth), materials))
        .collect();
    bricks.sort_unstable_by_key(|(key, _)| *key);

    // keep the whole tree in one page when it can fit; this bound ignores deduplication
    let worst_case_words = NODE_WORDS as u64 * (1 + bricks.len() as u64 * depth as u64)
        + BRICK_WORDS as u64 * bricks.len() as u64;
    arenas
        .topology
        .reserve_contiguous(worst_case_words.min(u32::MAX as u64) as u32)?;

    let mut builder = Builder {
        arenas,
        depth,
        pages: PageSpan::default(),
        stats: BakeStats::default(),
    };
    let root_record = if bricks.is_empty() {
        [0, ArenaHandle::INVALID.raw()]
    } else {
        builder.build(&bricks, 0)?
    };
    let root = builder.arenas.topology.alloc_dedup(&root_record)?;
    builder.pages.include(root.untyped());

    let bounds = module.occupied_bounds().map_or(Aabb::EMPTY, |(min, max)| {
        Aabb::new(min.as_vec3(), max.as_vec3())
    });
    let single_page = builder.pages.is_single(root.untyped().page());
    let stats = builder.stats;
    Ok((
        OctreeRoot {
            node: root.untyped(),
            depth: depth as u8,
            single_page,
            bounds,
        },
        stats,
    ))
}

/// The sort key of a brick: its octant at every level, most significant first, so sorted bricks
/// group by subtree and children appear in ascending octant order.
fn octant_key(coord: UVec3, depth: u32) -> u32 {
    let mut key = 0;
    for level in 0..depth {
        let shift = depth - 1 - level;
        let digit = ((coord.x >> shift) & 1)
            | ((coord.y >> shift) & 1) << 1
            | ((coord.z >> shift) & 1) << 2;
        key = (key << 3) | digit;
    }
    key
}

fn key_digit(key: u32, level: u32, depth: u32) -> u32 {
    (key >> (3 * (depth - 1 - level))) & 7
}

struct Builder<'a> {
    arenas: &'a mut GeometryArenas,
    depth: u32,
    pages: PageSpan,
    stats: BakeStats,
}

impl Builder<'_> {
    /// Builds the node at `level` covering `bricks`, returning its record.
    fn build(
        &mut self,
        bricks: &[(u32, BrickMaterials)],
        level: u32,
    ) -> Result<[u32; 2], ArenaError> {
        self.stats.nodes += 1;
        let brick_children = level == self.depth - 1;

        let mut mask = 0;
        let mut block = Vec::new();
        let mut start = 0;
        while start < bricks.len() {
            let digit = key_digit(bricks[start].0, level, self.depth);
            let mut end = start + 1;
            while end < bricks.len() && key_digit(bricks[end].0, level, self.depth) == digit {
                end += 1;
            }
            if brick_children {
                debug_assert_eq!(
                    end - start,
                    1,
                    "a last-level octant holds exactly one brick"
                );
                block.extend_from_slice(&self.brick_record(&bricks[start].1)?);
            } else {
                block.extend_from_slice(&self.build(&bricks[start..end], level + 1)?);
            }
            mask |= 1 << digit;
            start = end;
        }

        let handle = self.arenas.topology.alloc_dedup(&block)?;
        self.pages.include(handle.untyped());
        let tag = if brick_children { BRICK_CHILDREN } else { 0 };
        Ok([mask | tag, handle.untyped().raw()])
    }

    fn brick_record(
        &mut self,
        materials: &[MaterialId; BRICK_VOXELS],
    ) -> Result<[u32; 17], ArenaError> {
        let mut record = [0u32; BRICK_WORDS as usize];
        let mut attributes = Vec::new();
        for (k, &material) in materials.iter().enumerate() {
            if material != EMPTY {
                record[k / 32] |= 1 << (k % 32);
                attributes.push(material as u32);
            }
        }
        let handle = self.arenas.attributes.alloc_dedup(&attributes)?;
        record[BRICK_MASK_WORDS as usize] = handle.untyped().raw();
        self.stats.nodes += 1;
        self.stats.terminal_nodes += 1;
        self.stats.voxels += attributes.len() as u64;
        Ok(record)
    }
}

// INFO: -------------------
//         traversal
// -------------------------

/// One level of traversal state. On the brick level `child_ref` is the brick record.
#[derive(Clone, Copy, Default)]
struct Frame {
    mask: u32,
    child_ref: u32,
    origin: IVec3,
    cell: IVec3,
    t_next: Vec3,
}

fn read_node(geometry: &impl GeometrySource, handle: ArenaHandle) -> (u32, u32) {
    (
        geometry.topology_word(handle),
        geometry.topology_word(handle.offset(1)),
    )
}

/// Traces `ray` (in module voxel space) against `root` within `[t_min, t_max]`, returning the
/// nearest voxel hit.
pub fn trace(
    geometry: &impl GeometrySource,
    root: &OctreeRoot,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
) -> Option<VoxelHit> {
    trace_counted(geometry, root, ray, t_min, t_max).0
}

/// [`trace`], also returning the DDA steps taken, including on misses (for cost diagnostics).
pub fn trace_counted(
    geometry: &impl GeometrySource,
    root: &OctreeRoot,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
) -> (Option<VoxelHit>, u32) {
    let brick_level = root.depth as usize;
    let extent = root.extent() as i32;
    let (t0, t1) = slab(Vec3::ZERO, Vec3::splat(extent as f32), ray);
    let t_start = t0.max(t_min);
    let t_end = t1.min(t_max);
    if t_start > t_end {
        return (None, 0);
    }

    let (root_word, root_ref) = read_node(geometry, root.node);
    if root_word & CHILD_MASK == 0 {
        return (None, 0);
    }

    let step = step_of(ray);
    // a cell's far boundary along each axis is at `cell + 1` when stepping up, `cell` when down
    let far = (step + IVec3::ONE) / 2;
    let next_boundary = |origin: IVec3, cell: IVec3, size: i32| -> Vec3 {
        ((origin + (cell + far) * size).as_vec3() - ray.origin) * ray.inv_dir
    };
    let enter = |mask: u32, child_ref: u32, origin: IVec3, size: i32, cells: i32, t: f32| {
        let cell = entry_cell(origin, size, cells, ray, step, t);
        Frame {
            mask,
            child_ref,
            origin,
            cell,
            t_next: next_boundary(origin, cell, size),
        }
    };

    let mut stack = [Frame::default(); MAX_DEPTH as usize + 1];
    let mut level = 0;
    // size of a child cell at the current level, in voxels
    let mut size = extent / 2;
    stack[0] = enter(root_word, root_ref, IVec3::ZERO, size, 2, t_start);

    let mut steps = 0;
    while steps < MAX_TRAVERSAL_STEPS {
        steps += 1;
        let frame = stack[level];
        let cells = if level == brick_level { 8 } else { 2 };

        if frame.cell.cmplt(IVec3::ZERO).any() || frame.cell.cmpge(IVec3::splat(cells)).any() {
            // left this node or brick: resume the parent past the cell we were in
            if level == 0 {
                return (None, steps);
            }
            level -= 1;
            size = extent >> (level + 1);
            advance(&mut stack[level], step, &next_boundary, size);
            continue;
        }

        if level == brick_level {
            let k = brick_index(
                frame.cell.x as u32,
                frame.cell.y as u32,
                frame.cell.z as u32,
            );
            let brick = ArenaHandle::from_raw(frame.child_ref);
            let word = geometry.topology_word(brick.offset(k >> 5));
            if (word >> (k & 31)) & 1 != 0 {
                let lo = frame.origin + frame.cell;
                let (te, tx) = slab(lo.as_vec3(), (lo + IVec3::ONE).as_vec3(), ray);
                // a voxel; only report it if the ray genuinely touches it within range
                if te <= tx && tx >= t_start && te <= t_end {
                    let rank = (0..k >> 5)
                        .map(|w| geometry.topology_word(brick.offset(w)).count_ones())
                        .sum::<u32>()
                        + (word & ((1 << (k & 31)) - 1)).count_ones();
                    let attribute = ArenaHandle::from_raw(
                        geometry.topology_word(brick.offset(BRICK_MASK_WORDS)),
                    )
                    .offset(rank);
                    let word = geometry.attribute_word(attribute);
                    let local = lo.as_uvec3() % 4;
                    let hit = VoxelHit {
                        t: te.max(t_start),
                        cell: lo.as_uvec3(),
                        terminal_index: cell_index(local.x, local.y, local.z),
                        face: entry_face(lo, lo + IVec3::ONE, ray, step, te, t_start),
                        attribute,
                        material: (word & ATTRIBUTE_MATERIAL_MASK) as MaterialId,
                        lod: 0,
                        steps,
                    };
                    return (Some(hit), steps);
                }
            }
        } else {
            let k = (frame.cell.x + 2 * frame.cell.y + 4 * frame.cell.z) as u32;
            if frame.mask & (1 << k) != 0 {
                let lo = frame.origin + frame.cell * size;
                let (te, _) = slab(lo.as_vec3(), (lo + IVec3::splat(size)).as_vec3(), ray);
                let t = te.max(t_start);
                let rank = (frame.mask & ((1 << k) - 1)).count_ones();
                let children = ArenaHandle::from_raw(frame.child_ref);
                level += 1;
                if level == brick_level {
                    size = 1;
                    let brick = children.offset(rank * BRICK_WORDS);
                    stack[level] = enter(0, brick.raw(), lo, 1, 8, t);
                } else {
                    size /= 2;
                    let (mask, child_ref) = read_node(geometry, children.offset(rank * NODE_WORDS));
                    stack[level] = enter(mask, child_ref, lo, size, 2, t);
                }
                continue;
            }
        }

        advance(&mut stack[level], step, &next_boundary, size);
    }

    (None, steps)
}

/// Steps a frame into the next cell along the axis whose boundary is nearest.
fn advance(
    frame: &mut Frame,
    step: IVec3,
    next_boundary: &impl Fn(IVec3, IVec3, i32) -> Vec3,
    size: i32,
) {
    let axis = nearest_axis(frame.t_next);
    frame.cell[axis] += step[axis];
    frame.t_next = next_boundary(frame.origin, frame.cell, size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::HitFace;

    fn module_with(levels: u8, voxels: &[(u32, u32, u32)]) -> VoxelModule {
        let mut m = VoxelModule::new(levels);
        for &(x, y, z) in voxels {
            m.set(UVec3::new(x, y, z), 1 + (x % 5) as u16);
        }
        m
    }

    #[test]
    fn depth_covers_the_module() {
        assert_eq!(depth_for_extent(4), 1);
        assert_eq!(depth_for_extent(16), 1);
        assert_eq!(depth_for_extent(64), 3);
        assert_eq!(depth_for_extent(256), 5);
        assert_eq!(depth_for_extent(1024), MAX_DEPTH);
    }

    #[test]
    fn single_voxel_is_hit_from_every_axis() {
        let module = module_with(3, &[(37, 21, 50)]);
        let mut arenas = GeometryArenas::default();
        let (root, stats) = bake(&mut arenas, &module).unwrap();
        assert_eq!(stats.voxels, 1);
        assert_eq!(root.extent(), 64);
        assert!(root.single_page);

        let center = Vec3::new(37.5, 21.5, 50.5);
        for (dir, face) in [
            (Vec3::X, HitFace::NegX),
            (Vec3::NEG_X, HitFace::PosX),
            (Vec3::Y, HitFace::NegY),
            (Vec3::NEG_Y, HitFace::PosY),
            (Vec3::Z, HitFace::NegZ),
            (Vec3::NEG_Z, HitFace::PosZ),
        ] {
            let ray = Ray::new(center - dir * 100.0, dir);
            let hit = trace(&arenas, &root, &ray, 0.0, f32::MAX).expect("hit");
            assert_eq!(hit.cell, UVec3::new(37, 21, 50));
            assert_eq!(hit.face, face);
            assert_eq!(hit.material, module.get(hit.cell));
            assert_eq!(hit.terminal_index, cell_index(1, 1, 2));
            assert!((hit.t - 99.5).abs() < 1e-3);
        }
    }

    #[test]
    fn attribute_ranks_cross_occupancy_words() {
        // voxels spread over several occupancy words of one brick, each with its own material
        let voxels: Vec<(u32, u32, u32)> = (0..8).map(|i| (i, i, i)).collect();
        let module = module_with(2, &voxels);
        let mut arenas = GeometryArenas::default();
        let (root, _) = bake(&mut arenas, &module).unwrap();
        for &(x, y, z) in &voxels {
            let ray = Ray::new(Vec3::new(x as f32 + 0.5, y as f32 + 0.5, -10.0), Vec3::Z);
            let hit = trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap();
            assert_eq!(hit.cell, UVec3::new(x, y, z));
            assert_eq!(hit.material, module.get(hit.cell));
        }
    }

    #[test]
    fn tiny_modules_and_empty_modules() {
        let module = module_with(1, &[(3, 0, 2)]);
        let mut arenas = GeometryArenas::default();
        let (root, _) = bake(&mut arenas, &module).unwrap();
        assert_eq!(root.depth, 1);
        let ray = Ray::new(Vec3::new(3.5, 0.5, -1.0), Vec3::Z);
        assert_eq!(
            trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap().cell,
            UVec3::new(3, 0, 2)
        );

        let (empty, _) = bake(&mut arenas, &VoxelModule::new(3)).unwrap();
        assert!(trace(&arenas, &empty, &ray, 0.0, f32::MAX).is_none());
    }

    #[test]
    fn identical_bricks_are_shared() {
        let mut module = VoxelModule::new(3);
        for i in 0..4 {
            for (x, y, z) in [(0, 0, 0), (1, 6, 3), (7, 7, 0)] {
                module.set(UVec3::new(x + 16 * i, y, z), 2);
            }
        }
        let mut arenas = GeometryArenas::default();
        let (_, stats) = bake(&mut arenas, &module).unwrap();
        assert_eq!(stats.terminal_nodes, 4);
        assert_eq!(arenas.attributes.stats().blocks, 1);
    }
}
