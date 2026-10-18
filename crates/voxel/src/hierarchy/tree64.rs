//! The 64-way sparse voxel tree: bake and CPU reference traversal.
//!
//! Each node addresses a `4 x 4 x 4` grid of cells. A root of `levels` levels covers `4^levels`
//! voxels per axis; terminal nodes (depth `levels - 1`) cover one brick and their children are
//! individual voxels.
//!
//! Traversal is a hierarchical DDA. Inside a node it steps through child cells in ray order using
//! boundary distances recomputed from integer cell coordinates (never accumulated), and it keeps
//! one frame per level on a bounded stack. Entering a node picks the starting cell from those same
//! plane distances rather than from a reconstructed position, so the DDA and the slab test can
//! never disagree about which cells the ray passes through. The traversal mirrors
//! `assets/client/shader/voxel/lib/tree64.wesl` line for line; keep them in sync.

use crate::{
    arena::ArenaError,
    geometry::{GeometryArenas, GeometrySource},
    handle::ArenaHandle,
    lod::LodSummary,
    mask::{Mask64, cell_index},
    math::{Aabb, Ray, slab},
    module::{BRICK_VOXELS, Brick, EMPTY, MAX_LEVELS, MaterialId, VoxelModule},
    node::{NODE_WORDS, Node64},
};
use glam::{IVec3, UVec3, Vec3};

/// Traversal gives up (reporting a miss) after this many DDA steps, so a bad ray can never hang
/// the GPU. Diagnostics count rays that hit the cap.
pub const MAX_TRAVERSAL_STEPS: u32 = 4096;

/// Attribute word layout: material in the low 16 bits, the rest reserved for per-voxel data.
pub const ATTRIBUTE_MATERIAL_MASK: u32 = 0xFFFF;

/// A baked, immutable 64-tree root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tree64Root {
    /// The root node header (a one-node block in the topology arena).
    pub node: ArenaHandle,
    pub levels: u8,
    /// Whether every topology block reachable from the root lives in the root's logical page,
    /// letting traversal resolve the page once at root entry.
    pub single_page: bool,
    /// Occupied bounds in module voxel space.
    pub bounds: Aabb,
}

impl Tree64Root {
    pub fn extent(&self) -> u32 {
        4u32.pow(self.levels as u32)
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
    /// The voxel's 6-bit index within its terminal node.
    pub terminal_index: u32,
    pub face: HitFace,
    /// Logical handle of the hit voxel's attribute word.
    pub attribute: ArenaHandle,
    pub material: MaterialId,
    /// DDA steps taken, for diagnostics.
    pub steps: u32,
}

// INFO: --------------
//         bake
// --------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct BakeStats {
    pub nodes: u64,
    pub terminal_nodes: u64,
    pub voxels: u64,
}

/// Bakes `module` into `arenas`. See the [module docs](self).
pub fn bake(
    arenas: &mut GeometryArenas,
    module: &VoxelModule,
) -> Result<(Tree64Root, BakeStats), ArenaError> {
    let levels = module.levels() as u32;
    let mut bricks: Vec<(u32, &Brick)> = module
        .bricks()
        .iter()
        .map(|(&coord, brick)| (brick_key(coord, levels), brick))
        .collect();
    bricks.sort_unstable_by_key(|(key, _)| *key);

    // keep the whole tree in one page when it can fit, so traversal can take the single-page
    // fast path; this bound ignores deduplication and so is conservative
    let worst_case_words = (NODE_WORDS as u64) * (1 + bricks.len() as u64 * levels as u64);
    arenas
        .topology
        .reserve_contiguous(worst_case_words.min(u32::MAX as u64) as u32)?;

    let mut builder = Builder {
        arenas,
        levels,
        pages: PageSpan::default(),
        stats: BakeStats::default(),
    };

    let root_header = if bricks.is_empty() {
        Node64::internal(Mask64::EMPTY, ArenaHandle::INVALID, None)
    } else {
        builder.build(&bricks, 0)?.0
    };
    let root = builder
        .arenas
        .topology
        .alloc_dedup(&root_header.to_words())?;
    builder.pages.include(root.untyped());

    let bounds = module.occupied_bounds().map_or(Aabb::EMPTY, |(min, max)| {
        Aabb::new(min.as_vec3(), max.as_vec3())
    });
    let single_page = builder.pages.is_single(root.untyped().page());
    let stats = builder.stats;

    Ok((
        Tree64Root {
            node: root.untyped(),
            levels: levels as u8,
            single_page,
            bounds,
        },
        stats,
    ))
}

/// The sort key of a brick: its child index at every level, most significant first, so sorted
/// bricks group by subtree and children appear in ascending child index order.
fn brick_key(coord: UVec3, levels: u32) -> u32 {
    let mut key = 0;
    for depth in 0..levels.saturating_sub(1) {
        let shift = 2 * (levels - 2 - depth);
        let digit = cell_index(
            (coord.x >> shift) & 3,
            (coord.y >> shift) & 3,
            (coord.z >> shift) & 3,
        );
        key = (key << 6) | digit;
    }
    key
}

fn key_digit(key: u32, depth: u32, levels: u32) -> u32 {
    (key >> (6 * (levels - 2 - depth))) & 63
}

#[derive(Default)]
struct PageSpan {
    min: Option<u32>,
    max: u32,
}

impl PageSpan {
    fn include(&mut self, handle: ArenaHandle) {
        let page = handle.page();
        self.min = Some(self.min.map_or(page, |m| m.min(page)));
        self.max = self.max.max(page);
    }

    fn is_single(&self, page: u32) -> bool {
        self.min == Some(page) && self.max == page
    }
}

struct Builder<'a> {
    arenas: &'a mut GeometryArenas,
    levels: u32,
    pages: PageSpan,
    stats: BakeStats,
}

impl Builder<'_> {
    fn build(
        &mut self,
        bricks: &[(u32, &Brick)],
        depth: u32,
    ) -> Result<(Node64, Summary), ArenaError> {
        self.stats.nodes += 1;

        if depth == self.levels - 1 {
            debug_assert_eq!(bricks.len(), 1, "a terminal node covers exactly one brick");
            let brick = bricks[0].1;
            let mask = brick.occupancy();
            let attributes: Vec<u32> = mask
                .iter()
                .map(|k| brick.materials[k as usize] as u32)
                .collect();
            let summary = Summary::of_brick(brick);
            let lod = self.arenas.lod.push(summary.lod().pack());
            let handle = self.arenas.attributes.alloc_dedup(&attributes)?;
            self.stats.terminal_nodes += 1;
            self.stats.voxels += attributes.len() as u64;
            return Ok((Node64::terminal(mask, handle.untyped(), Some(lod)), summary));
        }

        let child_extent = 4u32.pow(self.levels - depth - 1);
        let mut mask = Mask64::EMPTY;
        let mut headers = Vec::new();
        let mut summary = Summary::empty(child_extent * 4);
        let mut start = 0;
        while start < bricks.len() {
            let digit = key_digit(bricks[start].0, depth, self.levels);
            let mut end = start + 1;
            while end < bricks.len() && key_digit(bricks[end].0, depth, self.levels) == digit {
                end += 1;
            }
            let (header, child_summary) = self.build(&bricks[start..end], depth + 1)?;
            mask.set(digit);
            headers.extend_from_slice(&header.to_words());
            summary.absorb(&child_summary, digit, child_extent);
            start = end;
        }

        let lod = self.arenas.lod.push(summary.lod().pack());
        let handle = self.arenas.topology.alloc_dedup(&headers)?;
        self.pages.include(handle.untyped());
        Ok((Node64::internal(mask, handle.untyped(), Some(lod)), summary))
    }
}

/// Bake-time aggregates used to fill a node's LOD record.
struct Summary {
    extent: u32,
    voxels: u64,
    /// Projections along X (onto y,z), Y (onto x,z), and Z (onto x,y).
    projections: [Bitmap; 3],
    /// Exposed face counts in `HitFace` order. Faces on a brick's border count as exposed, so
    /// this is an approximation at node boundaries.
    faces: [u64; 6],
    materials: Vec<(MaterialId, u64)>,
}

impl Summary {
    fn empty(extent: u32) -> Self {
        Self {
            extent,
            voxels: 0,
            projections: std::array::from_fn(|_| Bitmap::new(extent)),
            faces: [0; 6],
            materials: Vec::new(),
        }
    }

    fn of_brick(brick: &Brick) -> Self {
        let mut s = Self::empty(4);
        let occupied = |x: i32, y: i32, z: i32| {
            (0..4).contains(&x)
                && (0..4).contains(&y)
                && (0..4).contains(&z)
                && brick.materials[cell_index(x as u32, y as u32, z as u32) as usize] != EMPTY
        };
        for k in 0..BRICK_VOXELS as u32 {
            let material = brick.materials[k as usize];
            if material == EMPTY {
                continue;
            }
            let (x, y, z) = (k & 3, (k >> 2) & 3, k >> 4);
            s.voxels += 1;
            s.projections[0].set(y, z);
            s.projections[1].set(x, z);
            s.projections[2].set(x, y);
            let (xi, yi, zi) = (x as i32, y as i32, z as i32);
            let neighbors = [
                (xi - 1, yi, zi),
                (xi + 1, yi, zi),
                (xi, yi - 1, zi),
                (xi, yi + 1, zi),
                (xi, yi, zi - 1),
                (xi, yi, zi + 1),
            ];
            for (face, (nx, ny, nz)) in neighbors.into_iter().enumerate() {
                if !occupied(nx, ny, nz) {
                    s.faces[face] += 1;
                }
            }
            add_material(&mut s.materials, material, 1);
        }
        s
    }

    fn absorb(&mut self, child: &Summary, k: u32, child_extent: u32) {
        let (cx, cy, cz) = (k & 3, (k >> 2) & 3, k >> 4);
        let c = child_extent;
        self.voxels += child.voxels;
        self.projections[0].or_at(&child.projections[0], cy * c, cz * c);
        self.projections[1].or_at(&child.projections[1], cx * c, cz * c);
        self.projections[2].or_at(&child.projections[2], cx * c, cy * c);
        for (total, f) in self.faces.iter_mut().zip(child.faces) {
            *total += f;
        }
        for &(material, count) in &child.materials {
            add_material(&mut self.materials, material, count);
        }
        // only the dominant materials matter, so keep the list short
        if self.materials.len() > 8 {
            self.materials
                .sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            self.materials.truncate(8);
        }
    }

    fn lod(&self) -> LodSummary {
        let area = (self.extent as f64).powi(2);
        let f = self.faces.map(|c| c as f32);
        let normal = Vec3::new(f[1] - f[0], f[3] - f[2], f[5] - f[4]);
        let total: f32 = f.iter().sum();
        let (normal, spread) = if total > 0.0 {
            (normal.normalize_or_zero(), 1.0 - normal.length() / total)
        } else {
            (Vec3::ZERO, 1.0)
        };
        let material = self
            .materials
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .map_or(EMPTY, |m| m.0);
        LodSummary {
            volume: (self.voxels as f64 / (self.extent as f64).powi(3)) as f32,
            coverage: self
                .projections
                .each_ref()
                .map(|p| (p.count() as f64 / area) as f32),
            normal,
            spread,
            material,
        }
    }
}

fn add_material(list: &mut Vec<(MaterialId, u64)>, material: MaterialId, count: u64) {
    match list.iter_mut().find(|(m, _)| *m == material) {
        Some(entry) => entry.1 += count,
        None => list.push((material, count)),
    }
}

/// A square bitmap of `size x size` bits, row-major.
struct Bitmap {
    size: u32,
    words: Vec<u64>,
}

impl Bitmap {
    fn new(size: u32) -> Self {
        Self {
            size,
            words: vec![0; (size as usize * size as usize).div_ceil(64)],
        }
    }

    fn set(&mut self, u: u32, v: u32) {
        let i = (v * self.size + u) as usize;
        self.words[i / 64] |= 1 << (i % 64);
    }

    fn count(&self) -> u64 {
        self.words.iter().map(|w| w.count_ones() as u64).sum()
    }

    fn or_at(&mut self, child: &Bitmap, du: u32, dv: u32) {
        for (wi, &word) in child.words.iter().enumerate() {
            let mut bits = word;
            while bits != 0 {
                let i = wi as u32 * 64 + bits.trailing_zeros();
                bits &= bits - 1;
                self.set(du + i % child.size, dv + i / child.size);
            }
        }
    }
}

// INFO: -------------------
//         traversal
// -------------------------

fn read_node(geometry: &impl GeometrySource, handle: ArenaHandle) -> Node64 {
    Node64::from_words(std::array::from_fn(|i| {
        geometry.topology_word(handle.offset(i as u32))
    }))
}

/// One level of traversal state.
#[derive(Clone, Copy, Default)]
struct Frame {
    node: Node64,
    origin: IVec3,
    cell: IVec3,
    t_next: Vec3,
}

/// Traces `ray` (in module voxel space) against `root` within `[t_min, t_max]`, returning the
/// nearest voxel hit. See the [module docs](self).
pub fn trace(
    geometry: &impl GeometrySource,
    root: &Tree64Root,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
) -> Option<VoxelHit> {
    trace_counted(geometry, root, ray, t_min, t_max).0
}

/// [`trace`], also returning the DDA steps taken, including on misses (for cost diagnostics).
pub fn trace_counted(
    geometry: &impl GeometrySource,
    root: &Tree64Root,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
) -> (Option<VoxelHit>, u32) {
    let levels = root.levels as usize;
    let extent = root.extent() as i32;
    let (t0, t1) = slab(Vec3::ZERO, Vec3::splat(extent as f32), ray);
    let t_start = t0.max(t_min);
    let t_end = t1.min(t_max);
    if t_start > t_end {
        return (None, 0);
    }

    let root_node = read_node(geometry, root.node);
    if root_node.child_mask.is_empty() {
        return (None, 0);
    }

    let step = IVec3::new(
        if ray.dir.x < 0.0 { -1 } else { 1 },
        if ray.dir.y < 0.0 { -1 } else { 1 },
        if ray.dir.z < 0.0 { -1 } else { 1 },
    );
    // a cell's far boundary along each axis is at `cell + 1` when stepping up, `cell` when down
    let far = (step + IVec3::ONE) / 2;

    let next_boundary = |origin: IVec3, cell: IVec3, size: i32| -> Vec3 {
        ((origin + (cell + far) * size).as_vec3() - ray.origin) * ray.inv_dir
    };
    let enter = |node: Node64, origin: IVec3, size: i32, t: f32| -> Frame {
        let cell = entry_cell(origin, size, ray, step, t);
        Frame {
            node,
            origin,
            cell,
            t_next: next_boundary(origin, cell, size),
        }
    };

    let mut stack = [Frame::default(); MAX_LEVELS as usize];
    let mut depth = 0;
    // size of a child cell at the current depth, in voxels
    let mut size = extent / 4;
    stack[0] = enter(root_node, IVec3::ZERO, size, t_start);

    let mut steps = 0;
    while steps < MAX_TRAVERSAL_STEPS {
        steps += 1;
        let frame = stack[depth];

        if frame.cell.cmplt(IVec3::ZERO).any() || frame.cell.cmpgt(IVec3::splat(3)).any() {
            // left this node: resume the parent past the cell we were in
            if depth == 0 {
                return (None, steps);
            }
            depth -= 1;
            size *= 4;
            advance(&mut stack[depth], step, &next_boundary, size);
            continue;
        }

        let k = cell_index(
            frame.cell.x as u32,
            frame.cell.y as u32,
            frame.cell.z as u32,
        );
        if frame.node.child_mask.is_set(k) {
            let lo = frame.origin + frame.cell * size;
            let (te, tx) = slab(lo.as_vec3(), (lo + IVec3::splat(size)).as_vec3(), ray);

            if depth == levels - 1 {
                // a voxel; only report it if the ray genuinely touches it within range
                if te <= tx && tx >= t_start && te <= t_end {
                    let attribute = frame
                        .node
                        .child_ref()
                        .offset(frame.node.child_mask.prefix_count(k));
                    let word = geometry.attribute_word(attribute);
                    let hit = VoxelHit {
                        t: te.max(t_start),
                        cell: lo.as_uvec3(),
                        terminal_index: k,
                        face: entry_face(lo, size, ray, step, te, t_start),
                        attribute,
                        material: (word & ATTRIBUTE_MATERIAL_MASK) as MaterialId,
                        steps,
                    };
                    return (Some(hit), steps);
                }
            } else {
                let child_handle = frame
                    .node
                    .child_ref()
                    .offset(frame.node.child_mask.child_word_offset(k));
                let child = read_node(geometry, child_handle);
                depth += 1;
                size /= 4;
                stack[depth] = enter(child, lo, size, te.max(t_start));
                continue;
            }
        }

        advance(&mut stack[depth], step, &next_boundary, size);
    }

    (None, steps)
}

/// The child cell of a node that the ray occupies at `t`.
///
/// Cells are chosen by counting the node's internal planes the ray has crossed by `t`, using the
/// same `(plane - origin) * inv_dir` distances as the slab test. Reconstructing the entry point
/// and flooring it instead breaks for rays within an ulp of a plane: the rounded position lands
/// in the neighboring cell, and a ray running along that plane then misses everything beside it.
fn entry_cell(origin: IVec3, size: i32, ray: &Ray, step: IVec3, t: f32) -> IVec3 {
    let mut cell = IVec3::ZERO;
    for axis in 0..3 {
        let mut crossed = 0;
        for j in 1..4 {
            // planes in the order the ray meets them
            let plane = if step[axis] > 0 { j } else { 4 - j };
            let t_plane =
                ((origin[axis] + plane * size) as f32 - ray.origin[axis]) * ray.inv_dir[axis];
            if t_plane > t {
                break;
            }
            crossed += 1;
        }
        cell[axis] = if step[axis] > 0 { crossed } else { 3 - crossed };
    }
    cell
}

/// Steps a frame into the next cell along the axis whose boundary is nearest (ties pick the
/// lowest axis, matching the shader).
fn advance(
    frame: &mut Frame,
    step: IVec3,
    next_boundary: &impl Fn(IVec3, IVec3, i32) -> Vec3,
    size: i32,
) {
    let t = frame.t_next;
    let axis = if t.x <= t.y && t.x <= t.z {
        0
    } else if t.y <= t.z {
        1
    } else {
        2
    };
    frame.cell[axis] += step[axis];
    frame.t_next = next_boundary(frame.origin, frame.cell, size);
}

fn entry_face(lo: IVec3, size: i32, ray: &Ray, step: IVec3, te: f32, t_start: f32) -> HitFace {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn module_with(levels: u8, voxels: &[(u32, u32, u32)]) -> VoxelModule {
        let mut m = VoxelModule::new(levels);
        for &(x, y, z) in voxels {
            m.set(UVec3::new(x, y, z), 1);
        }
        m
    }

    #[test]
    fn single_voxel_is_hit_from_every_axis() {
        let module = module_with(3, &[(37, 21, 50)]);
        let mut arenas = GeometryArenas::default();
        let (root, stats) = arenas.bake_tree64(&module).unwrap();
        assert_eq!(stats.voxels, 1);
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
            assert_eq!(hit.material, 1);
            assert!((hit.t - 99.5).abs() < 1e-3);
        }
    }

    #[test]
    fn nearest_voxel_wins_and_ranges_clip() {
        let module = module_with(2, &[(2, 1, 1), (9, 1, 1)]);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();

        let forward = Ray::new(Vec3::new(-1.0, 1.5, 1.5), Vec3::X);
        assert_eq!(
            trace(&arenas, &root, &forward, 0.0, f32::MAX).unwrap().cell,
            UVec3::new(2, 1, 1)
        );
        let backward = Ray::new(Vec3::new(20.0, 1.5, 1.5), Vec3::NEG_X);
        assert_eq!(
            trace(&arenas, &root, &backward, 0.0, f32::MAX)
                .unwrap()
                .cell,
            UVec3::new(9, 1, 1)
        );
        // t_max stops short of the first voxel
        assert!(trace(&arenas, &root, &forward, 0.0, 2.5).is_none());
        // t_min skips past it
        assert_eq!(
            trace(&arenas, &root, &forward, 5.0, f32::MAX).unwrap().cell,
            UVec3::new(9, 1, 1)
        );
    }

    #[test]
    fn starting_inside_reports_inside() {
        let module = module_with(1, &[(1, 1, 1)]);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let ray = Ray::new(Vec3::new(1.5, 1.5, 1.5), Vec3::X);
        let hit = trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap();
        assert_eq!(hit.face, HitFace::Inside);
        assert_eq!(hit.t, 0.0);
    }

    #[test]
    fn identical_subtrees_are_shared() {
        // the same brick contents at many positions bake to one attribute block
        let mut module = VoxelModule::new(3);
        for i in 0..8 {
            for (x, y, z) in [(0, 0, 0), (1, 2, 3), (3, 3, 0)] {
                module.set(UVec3::new(x + 8 * i, y, z), 2);
            }
        }
        let mut arenas = GeometryArenas::default();
        let (_, stats) = arenas.bake_tree64(&module).unwrap();
        assert_eq!(stats.terminal_nodes, 8);
        assert_eq!(arenas.attributes.stats().blocks, 1);
        assert_eq!(arenas.attributes.stats().dedup_hits, 7);
    }

    #[test]
    fn empty_modules_never_hit() {
        let module = VoxelModule::new(3);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let ray = Ray::new(Vec3::new(-1.0, 3.0, 3.0), Vec3::X);
        assert!(trace(&arenas, &root, &ray, 0.0, f32::MAX).is_none());
    }

    #[test]
    fn lod_summaries_describe_their_subtree() {
        // a full 4x4 slab one voxel thick in z: half coverage along x/y, full along z
        let mut module = VoxelModule::new(1);
        for x in 0..4 {
            for y in 0..4 {
                module.set(UVec3::new(x, y, 0), 3);
            }
        }
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let header = read_node(&arenas, root.node);
        let lod = arenas.lod.get(header.lod_index().unwrap());
        assert_eq!(lod.volume, 0.25);
        assert_eq!(lod.coverage, [0.25, 0.25, 1.0]);
        assert_eq!(lod.material, 3);
    }
}
