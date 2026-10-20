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
//! never disagree about which cells the ray passes through (see [`super`]). The traversal mirrors
//! `assets/client/shader/voxel/lib/tree64.wesl` line for line; keep them in sync.
//!
//! [`trace_filtered`] adds footprint LOD filtering to the same loop, mirrored by
//! `trace_tree64_filtered` in the shader's `footprint_lod` variant.

use super::{
    ATTRIBUTE_MATERIAL_MASK, BakeStats, MAX_TRAVERSAL_STEPS, PageSpan, VoxelHit, entry_cell,
    entry_face, entry_face_box, nearest_axis, step_of,
};
use crate::{
    angular::{NodeVoxels, Sggx, bake_setup},
    arena::ArenaError,
    geometry::{GeometryArenas, GeometrySource},
    handle::ArenaHandle,
    lod::{FootprintFilter, LodSummary, OCCUPIED_STEPS},
    mask::{Mask64, cell_index},
    math::{Aabb, Ray, slab},
    module::{BRICK_VOXELS, Brick, EMPTY, MAX_LEVELS, MaterialId, VoxelModule},
    node::{NODE_WORDS, Node64},
};
use glam::{IVec3, UVec3, Vec3};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

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

// INFO: --------------
//         bake
// --------------------

/// Bakes `module` into `arenas`. See the [module docs](self).
pub fn bake(
    arenas: &mut GeometryArenas,
    module: &VoxelModule,
) -> Result<(Tree64Root, BakeStats), ArenaError> {
    let levels = module.levels() as u32;
    let mut bricks: Vec<(u32, UVec3, &Brick)> = module
        .bricks()
        .iter()
        .map(|(&coord, brick)| (brick_key(coord, levels), coord, brick))
        .collect();
    bricks.sort_unstable_by_key(|(key, _, _)| *key);

    // keep the whole tree in one page when it can fit, so traversal can take the single-page
    // fast path; this bound ignores deduplication and so is conservative
    let worst_case_words = (NODE_WORDS as u64) * (1 + bricks.len() as u64 * levels as u64);
    arenas
        .topology
        .reserve_contiguous(worst_case_words.min(u32::MAX as u64) as u32)?;

    let mut builder = Builder {
        arenas,
        module,
        levels,
        pages: PageSpan::default(),
        stats: BakeStats::default(),
        tensors: NodeTensors::measure(&bricks, levels),
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

struct Builder<'a> {
    arenas: &'a mut GeometryArenas,
    module: &'a VoxelModule,
    levels: u32,
    pages: PageSpan,
    stats: BakeStats,
    tensors: NodeTensors,
}

impl Builder<'_> {
    fn build(
        &mut self,
        bricks: &[(u32, UVec3, &Brick)],
        depth: u32,
    ) -> Result<(Node64, Summary), ArenaError> {
        self.stats.nodes += 1;

        if depth == self.levels - 1 {
            debug_assert_eq!(bricks.len(), 1, "a terminal node covers exactly one brick");
            let (_, coord, brick) = bricks[0];
            let mask = brick.occupancy();
            let attributes: Vec<u32> = mask
                .iter()
                .map(|k| brick.materials[k as usize] as u32)
                .collect();
            let summary = Summary::of_brick(brick, coord, self.module);
            let tensor = self.tensors.bricks[&mask.to_u64()];
            let lod = self.arenas.lod.push(summary.lod(tensor).pack());
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

        let tensor = self.tensors.nodes[&(depth, node_prefix(bricks[0].0, depth, self.levels))];
        let lod = self.arenas.lod.push(summary.lod(tensor).pack());
        let handle = self.arenas.topology.alloc_dedup(&headers)?;
        self.pages.include(handle.untyped());
        Ok((Node64::internal(mask, handle.untyped(), Some(lod)), summary))
    }
}

/// Every node's fitted directional tensor ([`crate::angular::BakeSetup::tensor`]), measured
/// before the build on all cores: the measurement dominates the bake.
struct NodeTensors {
    /// By brick occupancy: bricks repeat, and a brick's tensor depends only on its voxels.
    bricks: HashMap<u64, Sggx>,
    /// Internal nodes by depth and [`node_prefix`].
    nodes: HashMap<(u32, u32), Sggx>,
}

/// A node to measure: a distinct brick mask, or an internal node at a depth over a run of the
/// sorted bricks.
enum Measure {
    Brick(u64),
    Node(u32, usize, usize),
}

impl NodeTensors {
    fn measure(bricks: &[(u32, UVec3, &Brick)], levels: u32) -> Self {
        let mut jobs = Vec::new();
        for depth in 0..levels.saturating_sub(1) {
            let mut start = 0;
            while start < bricks.len() {
                let prefix = node_prefix(bricks[start].0, depth, levels);
                let mut end = start + 1;
                while end < bricks.len() && node_prefix(bricks[end].0, depth, levels) == prefix {
                    end += 1;
                }
                jobs.push(Measure::Node(depth, start, end));
                start = end;
            }
        }
        let mut masks: Vec<u64> = bricks.iter().map(|b| b.2.occupancy().to_u64()).collect();
        masks.sort_unstable();
        masks.dedup();
        jobs.extend(masks.into_iter().map(Measure::Brick));

        let setup = bake_setup();
        let measure = |job: &Measure| {
            match *job {
                Measure::Brick(mask) => NodeVoxels::from_brick_mask(mask),
                Measure::Node(depth, start, end) => {
                    let extent = 4u32.pow(levels - depth);
                    let corner = bricks[start].1 / (extent / 4) * (extent / 4);
                    NodeVoxels::from_bricks(
                        extent,
                        bricks[start..end]
                            .iter()
                            .map(|(_, coord, brick)| (coord - corner, brick.occupancy().to_u64())),
                    )
                }
            }
            .map_or(Sggx([0.0; 6]), |node| setup.tensor(&node))
        };

        // largest nodes first, so no thread is left with one at the end
        let next = AtomicUsize::new(0);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let mut tensors = vec![Sggx([0.0; 6]); jobs.len()];
        std::thread::scope(|scope| {
            let workers: Vec<_> = (0..threads.min(jobs.len()))
                .map(|_| {
                    scope.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            let Some(job) = jobs.get(i) else { break out };
                            out.push((i, measure(job)));
                        }
                    })
                })
                .collect();
            for worker in workers {
                for (i, tensor) in worker.join().expect("tensor measurement panicked") {
                    tensors[i] = tensor;
                }
            }
        });

        let mut out = Self {
            bricks: HashMap::new(),
            nodes: HashMap::new(),
        };
        for (job, tensor) in jobs.iter().zip(tensors) {
            match *job {
                Measure::Brick(mask) => {
                    out.bricks.insert(mask, tensor);
                }
                Measure::Node(depth, start, _) => {
                    out.nodes
                        .insert((depth, node_prefix(bricks[start].0, depth, levels)), tensor);
                }
            }
        }
        out
    }
}

/// The child digits above `depth` of a brick's sort key: the same for every brick of the node at
/// that depth.
fn node_prefix(key: u32, depth: u32, levels: u32) -> u32 {
    key.checked_shr(6 * (levels - 1 - depth)).unwrap_or(0)
}

/// Bake-time aggregates used to fill a node's LOD record.
struct Summary {
    extent: u32,
    voxels: u64,
    /// Each material's weight: `(exposed faces, voxels)`, compared in that order, so the material
    /// a summary shades with is the one covering most of the node's surface, not its interior.
    /// Faces are exposed when their neighbor voxel in the module is empty (neighbors in other
    /// bricks included).
    materials: Vec<(MaterialId, MaterialWeight)>,
    /// The occupied voxels' bounding box within the node, in voxels (`hi` exclusive).
    lo: UVec3,
    hi: UVec3,
}

type MaterialWeight = (u64, u64);

impl Summary {
    fn empty(extent: u32) -> Self {
        Self {
            extent,
            voxels: 0,
            materials: Vec::new(),
            lo: UVec3::splat(extent),
            hi: UVec3::ZERO,
        }
    }

    /// The summary of the brick at brick coordinate `coord` of `module`.
    fn of_brick(brick: &Brick, coord: UVec3, module: &VoxelModule) -> Self {
        let mut s = Self::empty(4);
        let corner = (coord * 4).as_ivec3();
        let occupied = |x: i32, y: i32, z: i32| {
            if (0..4).contains(&x) && (0..4).contains(&y) && (0..4).contains(&z) {
                return brick.materials[cell_index(x as u32, y as u32, z as u32) as usize] != EMPTY;
            }
            // across the brick's border: voxels outside the module are empty
            let p = corner + IVec3::new(x, y, z);
            p.min_element() >= 0 && module.get(p.as_uvec3()) != EMPTY
        };
        for k in 0..BRICK_VOXELS as u32 {
            let material = brick.materials[k as usize];
            if material == EMPTY {
                continue;
            }
            let (x, y, z) = (k & 3, (k >> 2) & 3, k >> 4);
            s.voxels += 1;
            s.lo = s.lo.min(UVec3::new(x, y, z));
            s.hi = s.hi.max(UVec3::new(x, y, z) + 1);
            let (xi, yi, zi) = (x as i32, y as i32, z as i32);
            let neighbors = [
                (xi - 1, yi, zi),
                (xi + 1, yi, zi),
                (xi, yi - 1, zi),
                (xi, yi + 1, zi),
                (xi, yi, zi - 1),
                (xi, yi, zi + 1),
            ];
            let exposed = neighbors
                .into_iter()
                .filter(|&(nx, ny, nz)| !occupied(nx, ny, nz))
                .count() as u64;
            add_material(&mut s.materials, material, (exposed, 1));
        }
        s
    }

    fn absorb(&mut self, child: &Summary, k: u32, child_extent: u32) {
        let (cx, cy, cz) = (k & 3, (k >> 2) & 3, k >> 4);
        let c = child_extent;
        self.voxels += child.voxels;
        let corner = UVec3::new(cx, cy, cz) * c;
        self.lo = self.lo.min(corner + child.lo);
        self.hi = self.hi.max(corner + child.hi);
        for &(material, weight) in &child.materials {
            add_material(&mut self.materials, material, weight);
        }
        // only the dominant materials matter, so keep the list short
        if self.materials.len() > 8 {
            self.materials
                .sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            self.materials.truncate(8);
        }
    }

    /// The node's record, with `tensor` its fitted directional coverage (relative to the occupied
    /// box, see [`crate::angular`]).
    fn lod(&self, tensor: Sggx) -> LodSummary {
        // the occupied box in sixteenths of the node's edge, rounded outward, as the record
        // stores it
        let sixteenth = self.extent as f64 / OCCUPIED_STEPS as f64;
        let box_lo = self.lo.as_dvec3() / sixteenth;
        let box_hi = self.hi.as_dvec3() / sixteenth;
        let (box_lo, box_hi) = (box_lo.floor(), box_hi.ceil());
        let material = self
            .materials
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .map_or(EMPTY, |m| m.0);
        LodSummary {
            volume: (self.voxels as f64 / (self.extent as f64).powi(3)) as f32,
            coverage: std::array::from_fn(|axis| tensor.0[axis].max(0.0).sqrt()),
            correlation: tensor.correlations(),
            material,
            occupied: [
                (box_lo / OCCUPIED_STEPS as f64).as_vec3(),
                (box_hi / OCCUPIED_STEPS as f64).as_vec3(),
            ],
        }
    }
}

fn add_material(
    list: &mut Vec<(MaterialId, MaterialWeight)>,
    material: MaterialId,
    weight: MaterialWeight,
) {
    match list.iter_mut().find(|(m, _)| *m == material) {
        Some(entry) => entry.1 = (entry.1.0 + weight.0, entry.1.1 + weight.1),
        None => list.push((material, weight)),
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
    trace_impl(geometry, root, ray, t_min, t_max, None)
}

/// [`trace_counted`] with footprint LOD filtering: an occupied node whose cell fits inside the
/// ray cone's footprint is resolved from its LOD summary instead of being descended (see
/// [`FootprintFilter`]). A filtered hit reports the node's aligned corner as `cell`, its index in
/// the parent as `terminal_index`, its level as `lod`, and the summary's material. The hit is
/// where the ray enters the summary's occupied box ([`LodSummary::filtered_box`]) clipped to the
/// root's occupied `bounds` (the bounds scene search culls frames with), and a ray that misses
/// the clipped box passes through. Nodes without
/// a summary are always descended, and the root is never filtered.
pub fn trace_filtered(
    geometry: &impl GeometrySource,
    root: &Tree64Root,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
    filter: &FootprintFilter,
) -> (Option<VoxelHit>, u32) {
    trace_impl(geometry, root, ray, t_min, t_max, Some(filter))
}

fn trace_impl(
    geometry: &impl GeometrySource,
    root: &Tree64Root,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
    filter: Option<&FootprintFilter>,
) -> (Option<VoxelHit>, u32) {
    let levels = root.levels as usize;
    let voxels_per_unit = ray.dir.length();
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

    let step = step_of(ray);
    // a cell's far boundary along each axis is at `cell + 1` when stepping up, `cell` when down
    let far = (step + IVec3::ONE) / 2;

    let next_boundary = |origin: IVec3, cell: IVec3, size: i32| -> Vec3 {
        ((origin + (cell + far) * size).as_vec3() - ray.origin) * ray.inv_dir
    };
    let enter = |node: Node64, origin: IVec3, size: i32, t: f32| -> Frame {
        let cell = entry_cell(origin, size, 4, ray, step, t);
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
                        face: entry_face(lo, lo + IVec3::splat(size), ray, step, te, t_start),
                        attribute,
                        material: (word & ATTRIBUTE_MATERIAL_MASK) as MaterialId,
                        lod: 0,
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
                if let Some(filter) = filter
                    && let Some(index) = child.lod_index()
                    && filter.covers(size as f32, te.max(t_start), voxels_per_unit)
                {
                    // the summary stands in for the whole subtree: hit it or pass through it.
                    // Its box is clipped to the module's occupied bounds, which scene search culls
                    // with, so a filtered surface never sticks out of its frame's bounds.
                    let summary = LodSummary::unpack(geometry.lod_record(index));
                    let (box_lo, box_hi) = summary.filtered_box(lo.as_vec3(), size as f32);
                    let clip_lo = box_lo.max(root.bounds.min);
                    let clip_hi = box_hi.min(root.bounds.max);
                    let (ce, cx) = slab(clip_lo, clip_hi, ray);
                    if ce <= cx
                        && cx >= t_start
                        && ce <= t_end
                        && filter.threshold < summary.opacity(ray.dir)
                    {
                        let hit = VoxelHit {
                            t: ce.max(t_start),
                            cell: lo.as_uvec3(),
                            terminal_index: k,
                            face: entry_face_box(clip_lo, clip_hi, ray, step, ce, t_start),
                            attribute: ArenaHandle::INVALID,
                            material: summary.material,
                            lod: (levels - 1 - depth) as u32,
                            steps,
                        };
                        return (Some(hit), steps);
                    }
                    advance(&mut stack[depth], step, &next_boundary, size);
                    continue;
                }
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

/// Steps a frame into the next cell along the axis whose boundary is nearest (ties pick the
/// lowest axis, matching the shader).
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
        // a full 4x4 slab one voxel thick in z: its occupied box is the slab, which it covers
        // from every axis
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
        assert!((lod.volume - 0.25).abs() < 1.0 / 255.0, "{}", lod.volume);
        for c in lod.coverage {
            assert!((c - 1.0).abs() < 1e-3, "{lod:?}");
        }
        assert_eq!(lod.material, 3);
        assert_eq!(lod.occupied, [Vec3::ZERO, Vec3::new(1.0, 1.0, 0.25)]);
    }

    #[test]
    fn baked_tensors_match_the_directional_study() {
        // the directional LOD study's baked SGGX must be the record the renderer reads
        let mut module = VoxelModule::new(2);
        for x in 0..16u32 {
            for z in 0..16u32 {
                if (x * 5 + z * 3) % 7 < 2 {
                    module.set(UVec3::new(x, (x + z) % 11, z), 1);
                }
            }
        }
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let lod = arenas
            .lod
            .get(read_node(&arenas, root.node).lod_index().unwrap());
        let node = crate::angular::NodeVoxels::from_module(&module, UVec3::ZERO, 16).unwrap();
        let (lo, hi) = node.occupied_box();
        assert_eq!(lod.occupied, [lo / 16.0, hi / 16.0]);
        let study = crate::lod::stored_tensor(crate::angular::bake_setup().tensor(&node));
        for (baked, measured) in lod.tensor().0.iter().zip(study.0) {
            assert!((baked - measured).abs() < 1e-6, "{baked} vs {measured}");
        }
        // the fit stays near the exact axis coverage
        for (baked, exact) in lod.coverage.iter().zip(node.axis_coverage()) {
            assert!((baked - exact).abs() < 0.15, "{baked} vs {exact}");
        }
    }

    #[test]
    fn baked_tensors_tell_diagonal_leaves_apart() {
        // two bricks, each a leaf along one diagonal: the axis coverages agree, the correlations
        // and so the opacities edge on and face on don't
        let mut module = VoxelModule::new(2);
        for i in 0..4 {
            for z in 0..4 {
                module.set(UVec3::new(i, i, z), 1);
                module.set(UVec3::new(4 + i, 3 - i, z), 1);
            }
        }
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let root_header = read_node(&arenas, root.node);
        let brick = |k: u32| {
            let header = read_node(
                &arenas,
                root_header
                    .child_ref()
                    .offset(root_header.child_mask.child_word_offset(k)),
            );
            arenas.lod.get(header.lod_index().unwrap())
        };
        let (a, b) = (brick(cell_index(0, 0, 0)), brick(cell_index(1, 0, 0)));
        let (along, across) = (Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, -1.0, 0.0));
        assert!(
            a.correlation[0] < -0.2 && b.correlation[0] > 0.2,
            "{a:?} {b:?}"
        );
        assert!(a.opacity(along) + 0.3 < a.opacity(across), "{a:?}");
        assert!(b.opacity(across) + 0.3 < b.opacity(along), "{b:?}");
    }

    #[test]
    fn lod_boxes_see_partial_bricks() {
        // a ground layer six voxels thick across the whole module: a brick at its top is half
        // full, and its occupied box is its bottom half
        let mut module = VoxelModule::new(2);
        for x in 0..16 {
            for y in 0..6 {
                for z in 0..16 {
                    module.set(UVec3::new(x, y, z), 1);
                }
            }
        }
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let root_header = read_node(&arenas, root.node);
        let child = cell_index(1, 1, 1);
        let header = read_node(
            &arenas,
            root_header
                .child_ref()
                .offset(root_header.child_mask.child_word_offset(child)),
        );
        let lod = arenas.lod.get(header.lod_index().unwrap());
        assert!((lod.volume - 0.5).abs() < 1.0 / 255.0, "{}", lod.volume);
        // the brick's top two rows are empty, and it fills its box from every direction
        assert_eq!(lod.occupied, [Vec3::ZERO, Vec3::new(1.0, 0.5, 1.0)]);
        for w in [Vec3::X, Vec3::Y, Vec3::ONE, Vec3::new(1.0, -2.0, 0.5)] {
            assert!((lod.opacity(w) - 1.0).abs() < 1e-3, "{w}: {lod:?}");
        }
    }

    fn filled_brick(module: &mut VoxelModule, corner: UVec3, material: MaterialId) {
        for i in 0..64 {
            module.set(corner + UVec3::new(i % 4, i / 4 % 4, i / 16), material);
        }
    }

    fn filter(radius: f32, threshold: f32) -> FootprintFilter {
        FootprintFilter {
            radius,
            spread: 0.0,
            threshold,
        }
    }

    #[test]
    fn wide_footprints_hit_the_lod_summary() {
        let mut module = VoxelModule::new(2);
        filled_brick(&mut module, UVec3::ZERO, 5);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let ray = Ray::new(Vec3::new(-10.0, 1.5, 1.5), Vec3::X);

        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &filter(2.0, 0.5))
            .0
            .unwrap();
        assert_eq!((hit.cell, hit.lod, hit.terminal_index), (UVec3::ZERO, 1, 0));
        assert_eq!((hit.face, hit.material), (HitFace::NegX, 5));
        assert!(!hit.attribute.is_valid());
        assert!((hit.t - 10.0).abs() < 1e-4);

        // a footprint narrower than the brick resolves the exact voxel
        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &filter(1.0, 0.5))
            .0
            .unwrap();
        assert_eq!(hit, trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap());
        assert_eq!((hit.cell, hit.lod), (UVec3::new(0, 1, 1), 0));
    }

    #[test]
    fn sparse_summaries_pass_rays_below_the_threshold() {
        // two voxels at opposite corners: the occupied box is the whole brick, and each axis'
        // projection covers 2/16 of its face
        let mut module = VoxelModule::new(2);
        module.set(UVec3::new(4, 0, 0), 2);
        module.set(UVec3::new(7, 3, 3), 2);
        filled_brick(&mut module, UVec3::new(8, 0, 0), 5);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let ray = Ray::new(Vec3::new(-10.0, 1.5, 1.5), Vec3::X);

        // a 0.5 cutoff sees through it
        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &filter(2.0, 0.5))
            .0
            .unwrap();
        assert_eq!(
            (hit.cell, hit.lod, hit.material),
            (UVec3::new(8, 0, 0), 1, 5)
        );
        // a lower threshold stops at it, and no filtering hits the voxel itself
        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &filter(2.0, 0.05))
            .0
            .unwrap();
        assert_eq!(
            (hit.cell, hit.lod, hit.material),
            (UVec3::new(4, 0, 0), 1, 2)
        );
        assert!((hit.t - 14.0).abs() < 1e-4, "t = {}", hit.t);
        assert_eq!(trace(&arenas, &root, &ray, 0.0, f32::MAX).unwrap().lod, 0);
    }

    #[test]
    fn filtered_hits_fill_only_the_occupied_box() {
        // one voxel at (5, 1, 1) in the brick at (4, 0, 0): its summary is that voxel's box,
        // which it covers fully, so the summary stands exactly where the voxel does
        let mut module = VoxelModule::new(2);
        module.set(UVec3::new(5, 1, 1), 2);
        module.set(UVec3::new(15, 15, 15), 1);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let wide = filter(2.0, 0.5);

        let ray = Ray::new(Vec3::new(-10.0, 1.5, 1.5), Vec3::X);
        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &wide)
            .0
            .unwrap();
        assert_eq!(
            (hit.cell, hit.lod, hit.face, hit.material),
            (UVec3::new(4, 0, 0), 1, HitFace::NegX, 2)
        );
        assert!((hit.t - 15.0).abs() < 1e-4, "t = {}", hit.t);
        // through the brick beside the voxel: no surface
        let beside = Ray::new(Vec3::new(-10.0, 2.5, 1.5), Vec3::X);
        assert_eq!(
            trace_filtered(&arenas, &root, &beside, 0.0, f32::MAX, &wide).0,
            None
        );
    }

    #[test]
    fn filtered_boxes_are_clipped_to_the_occupied_bounds() {
        // the brick at (4, 0, 0) is occupied only for x in 6..8 and y in 0..2
        let mut module = VoxelModule::new(2);
        for x in 6..8 {
            for y in 0..2 {
                for z in 0..4 {
                    module.set(UVec3::new(x, y, z), 3);
                }
            }
        }
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let wide = filter(2.0, 0.25);

        // the summary is entered at the bounds' face, not the brick's
        let ray = Ray::new(Vec3::new(-10.0, 1.5, 1.5), Vec3::X);
        let hit = trace_filtered(&arenas, &root, &ray, 0.0, f32::MAX, &wide)
            .0
            .unwrap();
        assert_eq!(
            (hit.cell, hit.lod, hit.face),
            (UVec3::new(4, 0, 0), 1, HitFace::NegX)
        );
        assert!((hit.t - 16.0).abs() < 1e-4, "t = {}", hit.t);
        // through the brick but outside the bounds: no surface
        let above = Ray::new(Vec3::new(-10.0, 3.0, 1.5), Vec3::X);
        assert_eq!(
            trace_filtered(&arenas, &root, &above, 0.0, f32::MAX, &wide).0,
            None
        );
    }

    #[test]
    fn footprints_grow_with_distance_and_ray_scale() {
        let mut module = VoxelModule::new(2);
        filled_brick(&mut module, UVec3::new(12, 0, 0), 5);
        let mut arenas = GeometryArenas::default();
        let (root, _) = arenas.bake_tree64(&module).unwrap();
        let cone = FootprintFilter {
            radius: 0.0,
            spread: 0.1,
            threshold: 0.5,
        };

        // diameter 2.4 at t = 12: too narrow for a 4-voxel brick
        let near = Ray::new(Vec3::new(0.0, 1.5, 1.5), Vec3::X);
        assert_eq!(
            trace_filtered(&arenas, &root, &near, 0.0, f32::MAX, &cone)
                .0
                .unwrap()
                .lod,
            0
        );
        // diameter 4.4 at t = 22
        let far = Ray::new(Vec3::new(-10.0, 1.5, 1.5), Vec3::X);
        assert_eq!(
            trace_filtered(&arenas, &root, &far, 0.0, f32::MAX, &cone)
                .0
                .unwrap()
                .lod,
            1
        );
        // a world-space radius covers twice as many voxels on a ray scaled 2x into the module
        let wide = filter(1.0, 0.5);
        assert_eq!(
            trace_filtered(&arenas, &root, &near, 0.0, f32::MAX, &wide)
                .0
                .unwrap()
                .lod,
            0
        );
        let scaled = Ray::new(Vec3::new(0.0, 1.5, 1.5), Vec3::X * 2.0);
        let hit = trace_filtered(&arenas, &root, &scaled, 0.0, f32::MAX, &wide)
            .0
            .unwrap();
        assert_eq!(hit.lod, 1);
        assert!((hit.t - 6.0).abs() < 1e-4);
    }
}
