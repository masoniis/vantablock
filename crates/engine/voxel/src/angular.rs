//! Directional opacity of LOD nodes: measured angular coverage and the compact representations
//! that approximate it (the spec's additional Phase 2 test, section 2.4's SGGX candidate).
//!
//! A filtered hit treats a node as a surface filling its occupied box (see [`crate::lod`]), so the
//! quantity a summary must predict is a node's **angular coverage**: the chance that a ray along
//! `w` that enters the occupied box hits one of the node's voxels. It depends only on the line, so
//! it is symmetric in `w`. [`NodeVoxels::coverage`] measures it with stratified parallel rays.
//! The candidates, all derived from the same voxels:
//!
//! - **Axis proxy** (the spec's): `clamp(sum(|w_a| * c_a), 0, 1)` over the exact box-relative
//!   axis coverages [`NodeVoxels::axis_coverage`].
//! - **SGGX tensor** ([`Sggx`]): a symmetric positive-semidefinite `S` with
//!   `sigma(w) = sqrt(max(0, w^T S w))`, six coefficients (12 bytes as `f16`). Built from the
//!   node's exposed faces (the usual surface construction, `S = sum(area * n n^T)`), it is diagonal
//!   for voxels, whose faces are axis aligned, and can't tell a leaf along one diagonal from a
//!   leaf along the other, so it is fitted here to measured coverage instead: least squares on
//!   `sigma^2 = coverage^2` over a fixed direction set, then projected onto the PSD cone. The
//!   diagonal tensor `diag(c_a^2)` is the axis coverages combined in `L2` instead of `L1`; fixing
//!   the fit's diagonal to it ([`Sggx::fit_with_axes`]) keeps most of the axis proxy's bias, so
//!   the bake fits all six terms.
//!
//! The bake ([`BakeSetup`], [`bake_setup`]) stores the fitted tensor in every LOD record
//! ([`crate::lod::LodSummary::tensor`]) from a cheaper measurement than the study's: 16
//! directions, bricks through precomputed line sets ([`BrickRays`]) and larger nodes with
//! `8 x 8` rays per direction.
//! - **Degree-4 even polynomial** ([`EvenPoly4`]): coverage fitted by the 15 homogeneous quartic
//!   monomials (spherical harmonic bands 0, 2, and 4), the richer reference representation.
//!
//! Projected area is not opacity (spec section 2.4): the fitted tensor's `sigma` is used as
//! coverage only because the fit maps it onto measured coverage.

use crate::{
    lod::{OCCUPIED_STEPS, f16_to_f32, f32_to_f16},
    mask::{Mask64, cell_coords, cell_index},
    module::VoxelModule,
};
use glam::{IVec3, UVec3, Vec3};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

/// The occupied voxels of one node, `size` voxels on a side, stored sparsely as the occupancy
/// masks of its `4^3` bricks (bit [`crate::mask::cell_index`]).
#[derive(Clone, Debug)]
pub struct NodeVoxels {
    size: u32,
    /// Occupancy by brick coordinate within the node; only nonempty bricks.
    bricks: HashMap<UVec3, u64>,
    /// The voxels' bounding box within the node (`hi` exclusive).
    lo: UVec3,
    hi: UVec3,
}

impl NodeVoxels {
    /// The node whose voxel `p` is occupied when `occupied(p)` holds; `None` when it is empty.
    pub fn from_fn(size: u32, occupied: impl Fn(UVec3) -> bool) -> Option<Self> {
        let mut node = Self::empty(size);
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    if occupied(UVec3::new(x, y, z)) {
                        node.set(UVec3::new(x, y, z));
                    }
                }
            }
        }
        node.finish()
    }

    /// The node of `module` with minimum corner `corner`, `size` voxels on a side (a multiple of
    /// the 4-voxel brick, `corner` aligned to it); `None` when it is empty.
    pub fn from_module(module: &VoxelModule, corner: UVec3, size: u32) -> Option<Self> {
        let bricks = size.div_ceil(4);
        let mut found = Vec::new();
        for bz in 0..bricks {
            for by in 0..bricks {
                for bx in 0..bricks {
                    let local = UVec3::new(bx, by, bz);
                    if let Some(brick) = module.bricks().get(&(corner / 4 + local)) {
                        found.push((local, brick.occupancy().to_u64()));
                    }
                }
            }
        }
        Self::from_bricks(size, found)
    }

    /// The node `size` voxels on a side made of `bricks`: occupancy masks by brick coordinate
    /// within the node. `None` when every mask is empty.
    pub fn from_bricks(size: u32, bricks: impl IntoIterator<Item = (UVec3, u64)>) -> Option<Self> {
        let mut node = Self::empty(size);
        for (coord, mask) in bricks {
            if mask == 0 {
                continue;
            }
            for k in Mask64::from_u64(mask).iter() {
                let (x, y, z) = cell_coords(k);
                let p = coord * 4 + UVec3::new(x, y, z);
                node.lo = node.lo.min(p);
                node.hi = node.hi.max(p + 1);
            }
            *node.bricks.entry(coord).or_default() |= mask;
        }
        node.finish()
    }

    /// The brick whose occupancy is `mask`; `None` when it is empty.
    pub fn from_brick_mask(mask: u64) -> Option<Self> {
        Self::from_bricks(4, [(UVec3::ZERO, mask)])
    }

    fn empty(size: u32) -> Self {
        Self {
            size,
            bricks: HashMap::new(),
            lo: UVec3::splat(size),
            hi: UVec3::ZERO,
        }
    }

    fn set(&mut self, p: UVec3) {
        *self.bricks.entry(p / 4).or_default() |= 1 << brick_bit(p);
        self.lo = self.lo.min(p);
        self.hi = self.hi.max(p + 1);
    }

    fn finish(self) -> Option<Self> {
        (self.lo.x < self.hi.x).then_some(self)
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn voxels(&self) -> u64 {
        self.bricks.values().map(|m| m.count_ones() as u64).sum()
    }

    pub fn get(&self, p: UVec3) -> bool {
        p.max_element() < self.size
            && self
                .bricks
                .get(&(p / 4))
                .is_some_and(|m| m >> brick_bit(p) & 1 != 0)
    }

    /// The occupied box in voxels, rounded outward to sixteenths of the node's edge exactly as the
    /// LOD record stores it.
    pub fn occupied_box(&self) -> (Vec3, Vec3) {
        let sixteenth = self.size as f64 / OCCUPIED_STEPS as f64;
        let lo = (self.lo.as_dvec3() / sixteenth).floor() * sixteenth;
        let hi = (self.hi.as_dvec3() / sixteenth).ceil() * sixteenth;
        (lo.as_vec3(), hi.as_vec3())
    }

    /// The fraction of the occupied box's face along each axis covered by the node's projection
    /// (exact: the measured coverage along the axes).
    pub fn axis_coverage(&self) -> [f32; 3] {
        let (lo, hi) = self.occupied_box();
        let ext = (hi - lo).as_dvec3();
        let faces = [ext.y * ext.z, ext.x * ext.z, ext.x * ext.y];
        let mut projections = [HashSet::new(), HashSet::new(), HashSet::new()];
        for (&coord, &mask) in &self.bricks {
            for k in Mask64::from_u64(mask).iter() {
                let (x, y, z) = cell_coords(k);
                let p = coord * 4 + UVec3::new(x, y, z);
                projections[0].insert((p.y, p.z));
                projections[1].insert((p.x, p.z));
                projections[2].insert((p.x, p.y));
            }
        }
        std::array::from_fn(|axis| (projections[axis].len() as f64 / faces[axis]).min(1.0) as f32)
    }

    /// The measured angular coverage along `dir`: the share of `samples x samples` stratified
    /// parallel rays over the occupied box's projection that, having entered the box, hit a voxel.
    pub fn coverage(&self, dir: Vec3, samples: u32) -> f32 {
        let w = dir.normalize();
        let (lo, hi) = self.occupied_box();
        let center = (lo + hi) * 0.5;
        let half = (hi - lo) * 0.5;
        let u = w.any_orthonormal_vector();
        let v = w.cross(u);
        let (eu, ev) = (half.dot(u.abs()), half.dot(v.abs()));
        let back = half.length() + 1.0;
        let (mut entered, mut hits) = (0u32, 0u32);
        for j in 0..samples {
            for i in 0..samples {
                let (ju, jv) = jitter(i, j);
                let a = (2.0 * (i as f32 + ju) / samples as f32 - 1.0) * eu;
                let b = (2.0 * (j as f32 + jv) / samples as f32 - 1.0) * ev;
                let origin = center + u * a + v * b - w * back;
                let inv = w.recip();
                let t0 = (lo - origin) * inv;
                let t1 = (hi - origin) * inv;
                let t_enter = t0.min(t1).max_element();
                let t_exit = t0.max(t1).min_element();
                if t_enter < t_exit {
                    entered += 1;
                    if self.march(origin, w, t_enter, t_exit, lo, hi) {
                        hits += 1;
                    }
                }
            }
        }
        if entered == 0 {
            0.0
        } else {
            hits as f32 / entered as f32
        }
    }

    /// Whether the segment `[t_enter, t_exit]` of the ray hits a voxel inside the box: a voxel DDA
    /// that jumps over empty bricks (a measurement, not a traversal that reports identity).
    fn march(&self, origin: Vec3, w: Vec3, t_enter: f32, t_exit: f32, lo: Vec3, hi: Vec3) -> bool {
        let (lo, hi) = (lo.as_ivec3(), hi.as_ivec3());
        let step = IVec3::new(sign(w.x), sign(w.y), sign(w.z));
        let inv = w.recip();
        let delta = inv.abs();
        let next = |cell: IVec3, axis: usize| {
            if w[axis] == 0.0 {
                f32::INFINITY
            } else {
                let plane = cell[axis] + (step[axis] > 0) as i32;
                (plane as f32 - origin[axis]) * inv[axis]
            }
        };
        let mut t = t_enter + (t_exit - t_enter) * 1e-4;
        loop {
            let mut cell = (origin + w * t).floor().as_ivec3().clamp(lo, hi - 1);
            let mut t_next = Vec3::new(next(cell, 0), next(cell, 1), next(cell, 2));
            loop {
                let brick: IVec3 = cell >> 2;
                let Some(&mask) = self.bricks.get(&brick.as_uvec3()) else {
                    // leave the empty brick and resume from where the ray exits it
                    let b_lo = (brick * 4).as_vec3();
                    let t0 = (b_lo - origin) * inv;
                    let t1 = (b_lo + 4.0 - origin) * inv;
                    let t_out = t0.max(t1).min_element();
                    if t_out >= t_exit {
                        return false;
                    }
                    t = t.max(t_out) + 1e-3;
                    break;
                };
                // step through the brick's voxels
                loop {
                    if mask >> brick_bit(cell.as_uvec3()) & 1 != 0 {
                        return true;
                    }
                    let axis = if t_next.x <= t_next.y && t_next.x <= t_next.z {
                        0
                    } else if t_next.y <= t_next.z {
                        1
                    } else {
                        2
                    };
                    if t_next[axis] > t_exit {
                        return false;
                    }
                    cell[axis] += step[axis];
                    if cell[axis] < lo[axis] || cell[axis] >= hi[axis] {
                        return false;
                    }
                    t_next[axis] += delta[axis];
                    if (cell >> 2) != brick {
                        break;
                    }
                }
            }
        }
    }
}

/// The bit of voxel `p` in its brick's occupancy mask.
fn brick_bit(p: UVec3) -> u32 {
    cell_index(p.x & 3, p.y & 3, p.z & 3)
}

fn sign(v: f32) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

/// A stable jitter in `[0, 1)^2` for stratum `(i, j)` (PCG hash).
fn jitter(i: u32, j: u32) -> (f32, f32) {
    let hash = |mut v: u32| {
        v = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
        let w = ((v >> ((v >> 28) + 4)) ^ v).wrapping_mul(277_803_737);
        (w >> 22) ^ w
    };
    let a = hash(i ^ hash(j));
    let b = hash(a);
    (
        (a >> 8) as f32 / (1u32 << 24) as f32,
        (b >> 8) as f32 / (1u32 << 24) as f32,
    )
}

/// `n` directions spread over the `+Z` hemisphere (a Fibonacci spiral rotated by `phase`
/// radians). Coverage is symmetric in the direction, so a hemisphere covers every line.
pub fn hemisphere_directions(n: usize, phase: f32) -> Vec<Vec3> {
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    (0..n)
        .map(|i| {
            let z = (i as f32 + 0.5) / n as f32;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let phi = i as f32 * golden + phase;
            Vec3::new(r * phi.cos(), r * phi.sin(), z)
        })
        .collect()
}

/// The spec's `L1` axis-coverage opacity proxy (the renderer evaluates the fitted tensor,
/// [`crate::lod::LodSummary::opacity`]).
pub fn axis_opacity(coverage: [f32; 3], dir: Vec3) -> f32 {
    dir.normalize_or_zero()
        .abs()
        .dot(Vec3::from(coverage))
        .clamp(0.0, 1.0)
}

/// How the bake measures directional coverage for a node's fitted tensor: `directions` over the
/// hemisphere, `samples x samples` stratified rays each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakeMeasurement {
    pub directions: usize,
    pub samples: u32,
    /// [`BrickRays`]' lines per direction: `brick_grid x brick_grid`.
    pub brick_grid: u32,
}

/// The measurement the bake uses ([`bake_setup`]).
pub const BAKE_MEASUREMENT: BakeMeasurement = BakeMeasurement {
    directions: 16,
    samples: 8,
    brick_grid: 64,
};

/// A [`BakeMeasurement`] ready to measure nodes: its directions and brick line sets.
pub struct BakeSetup {
    pub measurement: BakeMeasurement,
    pub directions: Vec<Vec3>,
    pub bricks: BrickRays,
}

impl BakeSetup {
    pub fn new(measurement: BakeMeasurement) -> Self {
        let directions = hemisphere_directions(measurement.directions, 0.0);
        Self {
            measurement,
            bricks: BrickRays::new(&directions, measurement.brick_grid),
            directions,
        }
    }

    /// The tensor the bake stores for `node`: its coverage measured at [`Self::directions`] and
    /// fitted ([`Sggx::fit`]). Bricks are measured through [`BrickRays`], larger nodes with
    /// `samples x samples` rays per direction ([`NodeVoxels::coverage`]).
    pub fn tensor(&self, node: &NodeVoxels) -> Sggx {
        let coverage: Vec<f32> = if node.size == 4 {
            let mask = node.bricks.get(&UVec3::ZERO).copied().unwrap_or(0);
            self.bricks.coverage(mask, node.lo, node.hi)
        } else {
            self.directions
                .iter()
                .map(|&w| node.coverage(w, self.measurement.samples))
                .collect()
        };
        Sggx::fit(&self.directions, &coverage)
    }
}

/// The setup of [`BAKE_MEASUREMENT`], built once per process.
pub fn bake_setup() -> &'static BakeSetup {
    static SETUP: OnceLock<BakeSetup> = OnceLock::new();
    SETUP.get_or_init(|| BakeSetup::new(BAKE_MEASUREMENT))
}

/// Every distinct set of brick voxels a line along each direction passes through, weighted by
/// the share of the brick's projection whose lines pass through exactly that set. A brick's
/// coverage along a direction is then a weighted count of the sets it intersects, so measuring a
/// brick costs a few hundred mask tests per direction instead of a DDA per ray.
pub struct BrickRays {
    /// Per direction: `(voxel mask, weight)`, nonempty masks only.
    sets: Vec<Vec<(u64, u32)>>,
}

impl BrickRays {
    /// The sets of `dirs`, from `grid x grid` jittered lines over the brick's projection.
    pub fn new(dirs: &[Vec3], grid: u32) -> Self {
        let sets = dirs
            .iter()
            .map(|&dir| {
                let w = dir.normalize();
                let center = Vec3::splat(2.0);
                let u = w.any_orthonormal_vector();
                let v = w.cross(u);
                let half = Vec3::splat(2.0);
                let (eu, ev) = (half.dot(u.abs()), half.dot(v.abs()));
                let mut counts = std::collections::HashMap::<u64, u32>::new();
                for j in 0..grid {
                    for i in 0..grid {
                        let (ju, jv) = jitter(i, j);
                        let a = (2.0 * (i as f32 + ju) / grid as f32 - 1.0) * eu;
                        let b = (2.0 * (j as f32 + jv) / grid as f32 - 1.0) * ev;
                        let mask = line_voxels(center + u * a + v * b, w);
                        if mask != 0 {
                            *counts.entry(mask).or_default() += 1;
                        }
                    }
                }
                let mut sets: Vec<(u64, u32)> = counts.into_iter().collect();
                sets.sort_unstable();
                sets
            })
            .collect();
        Self { sets }
    }

    /// The number of sets over every direction (the cost of measuring one brick).
    pub fn len(&self) -> usize {
        self.sets.iter().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The coverage per direction of the brick whose occupancy is `mask`, with voxel bounds
    /// `lo..hi` (its occupied box).
    fn coverage(&self, mask: u64, lo: UVec3, hi: UVec3) -> Vec<f32> {
        let mut bounds = 0u64;
        for z in lo.z..hi.z {
            for y in lo.y..hi.y {
                for x in lo.x..hi.x {
                    bounds |= 1 << (x + 4 * y + 16 * z);
                }
            }
        }
        self.sets
            .iter()
            .map(|sets| {
                let (mut hits, mut entered) = (0u32, 0u32);
                for &(set, weight) in sets {
                    if set & bounds != 0 {
                        entered += weight;
                        if set & mask != 0 {
                            hits += weight;
                        }
                    }
                }
                if entered == 0 {
                    0.0
                } else {
                    hits as f32 / entered as f32
                }
            })
            .collect()
    }
}

/// The voxels of a `4^3` brick whose interiors the line through `p` along `w` crosses.
fn line_voxels(p: Vec3, w: Vec3) -> u64 {
    let inv = w.recip();
    let mut mask = 0;
    for k in 0..64u32 {
        let lo = Vec3::new((k & 3) as f32, ((k >> 2) & 3) as f32, (k >> 4) as f32);
        let t0 = (lo - p) * inv;
        let t1 = (lo + 1.0 - p) * inv;
        if t0.min(t1).max_element() < t0.max(t1).min_element() {
            mask |= 1 << k;
        }
    }
    mask
}

/// A symmetric projected-area tensor (SGGX): `xx, yy, zz, xy, xz, yz`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sggx(pub [f32; 6]);

impl Sggx {
    /// `diag(c_a^2)`: the axis coverages combined in `L2`, exact along the axes.
    pub fn from_axis_coverage(c: [f32; 3]) -> Self {
        Self([c[0] * c[0], c[1] * c[1], c[2] * c[2], 0.0, 0.0, 0.0])
    }

    /// The PSD tensor whose `sigma^2` best fits `coverage^2` at `dirs` (least squares, then
    /// negative eigenvalues clamped to zero).
    pub fn fit(dirs: &[Vec3], coverage: &[f32]) -> Self {
        let rows: Vec<Vec<f64>> = dirs.iter().map(|&w| quadratic(w).to_vec()).collect();
        let targets: Vec<f64> = coverage.iter().map(|&c| (c as f64).powi(2)).collect();
        let s = least_squares(&rows, &targets);
        let m = [[s[0], s[3], s[4]], [s[3], s[1], s[5]], [s[4], s[5], s[2]]];
        let m = psd_projection(m);
        Self([m[0][0], m[1][1], m[2][2], m[0][1], m[0][2], m[1][2]].map(|v| v as f32))
    }

    /// The PSD tensor with diagonal `diag(axis^2)` (exact along the axes) whose off-diagonal
    /// terms best fit `coverage^2` at `dirs`: least squares on the correlations
    /// `r_ab = S_ab / (c_a c_b)`, clamped to `[-1, 1]` and shrunk toward zero until the
    /// correlation matrix is positive semidefinite.
    pub fn fit_with_axes(axis: [f32; 3], dirs: &[Vec3], coverage: &[f32]) -> Self {
        let c = Vec3::from(axis).as_dvec3();
        let (rows, targets): (Vec<Vec<f64>>, Vec<f64>) = dirs
            .iter()
            .zip(coverage)
            .map(|(&w, &m)| {
                let q = quadratic(w);
                let diagonal = q[0] * c.x * c.x + q[1] * c.y * c.y + q[2] * c.z * c.z;
                (
                    vec![q[3] * c.x * c.y, q[4] * c.x * c.z, q[5] * c.y * c.z],
                    (m as f64).powi(2) - diagonal,
                )
            })
            .unzip();
        let r = least_squares(&rows, &targets);
        Self::from_axes_and_correlations(
            axis,
            psd_correlations([r[0], r[1], r[2]]).map(|v| v as f32),
        )
    }

    /// `S_aa = c_a^2`, `S_ab = r_ab c_a c_b` (correlations in `xy, xz, yz` order).
    pub fn from_axes_and_correlations(axis: [f32; 3], r: [f32; 3]) -> Self {
        let [x, y, z] = axis;
        Self([
            x * x,
            y * y,
            z * z,
            r[0] * x * y,
            r[1] * x * z,
            r[2] * y * z,
        ])
    }

    /// The correlations `r_ab = S_ab / sqrt(S_aa S_bb)` in `xy, xz, yz` order (`0` where a
    /// diagonal term is zero).
    pub fn correlations(&self) -> [f32; 3] {
        let s = self.0;
        let r = |ab: f32, aa: f32, bb: f32| {
            let d = (aa * bb).sqrt();
            if d > 0.0 {
                (ab / d).clamp(-1.0, 1.0)
            } else {
                0.0
            }
        };
        [
            r(s[3], s[0], s[1]),
            r(s[4], s[0], s[2]),
            r(s[5], s[1], s[2]),
        ]
    }

    /// `sigma(w) = sqrt(max(0, w^T S w))` for the normalized direction.
    pub fn sigma(&self, dir: Vec3) -> f32 {
        let w = dir.normalize_or_zero();
        let s = self.0;
        let q = s[0] * w.x * w.x
            + s[1] * w.y * w.y
            + s[2] * w.z * w.z
            + 2.0 * (s[3] * w.x * w.y + s[4] * w.x * w.z + s[5] * w.y * w.z);
        q.max(0.0).sqrt()
    }

    /// `sigma` read as coverage, clamped to `[0, 1]`.
    pub fn opacity(&self, dir: Vec3) -> f32 {
        self.sigma(dir).min(1.0)
    }

    /// The tensor after an `f16` round trip, as six packed halves (12 bytes) store it.
    pub fn quantized(&self) -> Self {
        Self(self.0.map(|v| f16_to_f32(f32_to_f16(v))))
    }
}

/// `w^T S w`'s monomials in `Sggx` order (cross terms doubled).
fn quadratic(w: Vec3) -> [f64; 6] {
    let w = w.normalize().as_dvec3();
    [
        w.x * w.x,
        w.y * w.y,
        w.z * w.z,
        2.0 * w.x * w.y,
        2.0 * w.x * w.z,
        2.0 * w.y * w.z,
    ]
}

/// Coverage as a homogeneous quartic of the unit direction (15 coefficients: spherical harmonic
/// bands 0, 2, and 4).
#[derive(Clone, Debug, PartialEq)]
pub struct EvenPoly4(pub [f32; 15]);

impl EvenPoly4 {
    pub fn fit(dirs: &[Vec3], coverage: &[f32]) -> Self {
        let rows: Vec<Vec<f64>> = dirs.iter().map(|&w| quartic(w).to_vec()).collect();
        let targets: Vec<f64> = coverage.iter().map(|&c| c as f64).collect();
        let c = least_squares(&rows, &targets);
        Self(std::array::from_fn(|i| c[i] as f32))
    }

    pub fn opacity(&self, dir: Vec3) -> f32 {
        let m = quartic(dir);
        let v: f64 = m.iter().zip(self.0).map(|(m, c)| m * c as f64).sum();
        (v as f32).clamp(0.0, 1.0)
    }
}

fn quartic(w: Vec3) -> [f64; 15] {
    let w = w.normalize().as_dvec3();
    let mut out = [0.0; 15];
    let mut n = 0;
    for i in 0..=4 {
        for j in 0..=4 - i {
            out[n] = w.x.powi(i) * w.y.powi(j) * w.z.powi(4 - i - j);
            n += 1;
        }
    }
    out
}

/// Least squares through the normal equations (Gaussian elimination with partial pivoting and a
/// tiny ridge, so underdetermined fits stay finite).
fn least_squares(rows: &[Vec<f64>], targets: &[f64]) -> Vec<f64> {
    let n = rows[0].len();
    let mut a = vec![vec![0.0; n + 1]; n];
    for (row, &t) in rows.iter().zip(targets) {
        for i in 0..n {
            for j in 0..n {
                a[i][j] += row[i] * row[j];
            }
            a[i][n] += row[i] * t;
        }
    }
    for (i, r) in a.iter_mut().enumerate() {
        r[i] += 1e-9;
    }
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))
            .unwrap();
        a.swap(col, pivot);
        let pivot_row = a[col].clone();
        for row in &mut a[col + 1..] {
            let f = row[col] / pivot_row[col];
            for (v, p) in row[col..].iter_mut().zip(&pivot_row[col..]) {
                *v -= f * p;
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|j| a[i][j] * x[j]).sum();
        x[i] = (a[i][n] - s) / a[i][i];
    }
    x
}

/// Correlations clamped to `[-1, 1]` and scaled toward zero (bisection) until the unit-diagonal
/// matrix they form is positive semidefinite (`det >= 0`; its 2x2 minors already are).
fn psd_correlations(r: [f64; 3]) -> [f64; 3] {
    let r = r.map(|v| v.clamp(-1.0, 1.0));
    let det = |k: f64| {
        let [a, b, c] = r.map(|v| v * k);
        1.0 + 2.0 * a * b * c - a * a - b * b - c * c
    };
    if det(1.0) >= 0.0 {
        return r;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..32 {
        let mid = 0.5 * (lo + hi);
        if det(mid) >= 0.0 { lo = mid } else { hi = mid }
    }
    r.map(|v| v * lo)
}

/// The nearest positive-semidefinite matrix to the symmetric `m` (Jacobi eigendecomposition,
/// negative eigenvalues clamped to zero).
fn psd_projection(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut a = m;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..64 {
        let (p, q) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|&(a0, b0), &(a1, b1)| a[a0][b0].abs().total_cmp(&a[a1][b1].abs()))
            .unwrap();
        if a[p][q].abs() < 1e-15 {
            break;
        }
        let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
        let c = 1.0 / (t * t + 1.0).sqrt();
        let s = t * c;
        for row in &mut a {
            let (akp, akq) = (row[p], row[q]);
            row[p] = c * akp - s * akq;
            row[q] = s * akp + c * akq;
        }
        let (rp, rq) = (a[p], a[q]);
        a[p] = std::array::from_fn(|k| c * rp[k] - s * rq[k]);
        a[q] = std::array::from_fn(|k| s * rp[k] + c * rq[k]);
        for row in &mut v {
            let (vkp, vkq) = (row[p], row[q]);
            row[p] = c * vkp - s * vkq;
            row[q] = s * vkp + c * vkq;
        }
    }
    let l = [a[0][0].max(0.0), a[1][1].max(0.0), a[2][2].max(0.0)];
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| v[i][k] * l[k] * v[j][k]).sum()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A leaf one voxel thick along the plane `x == y`, in a 16-voxel node.
    fn diagonal_leaf() -> NodeVoxels {
        NodeVoxels::from_fn(16, |p| p.x == p.y && (2..14).contains(&p.z)).unwrap()
    }

    #[test]
    fn solid_nodes_cover_every_direction() {
        let node = NodeVoxels::from_fn(16, |p| p.y < 8).unwrap();
        assert_eq!(node.axis_coverage(), [1.0, 1.0, 1.0]);
        for w in hemisphere_directions(16, 0.3) {
            assert_eq!(node.coverage(w, 16), 1.0);
        }
        assert!(NodeVoxels::from_fn(4, |_| false).is_none());
    }

    #[test]
    fn axis_directions_measure_axis_coverage() {
        // scattered voxels: the measured coverage along an axis is the projected share
        let node = NodeVoxels::from_fn(16, |p| (p.x * 7 + p.y * 3 + p.z * 5) % 11 == 0).unwrap();
        let axis = node.axis_coverage();
        for (a, w) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().enumerate() {
            let measured = node.coverage(w, 64);
            assert!(
                (measured - axis[a]).abs() < 0.03,
                "{a}: {measured} vs {}",
                axis[a]
            );
        }
    }

    #[test]
    fn diagonal_leaves_defeat_the_axis_proxy_but_not_the_fitted_tensor() {
        let node = diagonal_leaf();
        let axis = node.axis_coverage();
        let along = Vec3::new(1.0, 1.0, 0.0).normalize();
        let across = Vec3::new(1.0, -1.0, 0.0).normalize();
        // seen edge on it covers little of its box, seen face on all of it
        assert!(node.coverage(along, 64) < 0.2);
        assert!(node.coverage(across, 64) > 0.95);
        assert_eq!(axis_opacity(axis, along), 1.0);
        let dirs = hemisphere_directions(48, 0.0);
        let measured: Vec<f32> = dirs.iter().map(|&w| node.coverage(w, 32)).collect();
        let sggx = Sggx::fit(&dirs, &measured).quantized();
        assert!(sggx.opacity(along) < 0.4, "{}", sggx.opacity(along));
        assert!(sggx.opacity(across) > 0.8, "{}", sggx.opacity(across));
        // the face construction is diagonal and can't tell the two apart
        let diagonal = Sggx::from_axis_coverage(axis);
        assert_eq!(diagonal.opacity(along), diagonal.opacity(across));
    }

    #[test]
    fn brick_line_sets_measure_what_rays_do() {
        let setup = bake_setup();
        for mask in [0x8421_8421_0F0F_1248u64, 0x0000_0000_0060_0660, !0, 1 << 21] {
            let node = NodeVoxels::from_brick_mask(mask).unwrap();
            let sets = setup.bricks.coverage(mask, node.lo, node.hi);
            for (&w, set) in setup.directions.iter().zip(sets) {
                let rays = node.coverage(w, 96);
                assert!((set - rays).abs() < 0.03, "{mask:x} {w}: {set} vs {rays}");
            }
        }
    }

    #[test]
    fn fits_recover_representable_functions() {
        let dirs = hemisphere_directions(48, 0.0);
        // a PSD tensor's own sigma
        let truth = Sggx([0.6, 0.3, 0.5, -0.2, 0.1, 0.05]);
        let sigma: Vec<f32> = dirs.iter().map(|&w| truth.sigma(w)).collect();
        let fitted = Sggx::fit(&dirs, &sigma);
        for (a, b) in fitted.0.iter().zip(truth.0) {
            assert!((a - b).abs() < 1e-4, "{fitted:?}");
        }
        // an indefinite fit is projected onto the PSD cone: a negative eigenvalue becomes zero
        let projected = psd_projection([[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 0.5]]);
        assert!((projected[1][1]).abs() < 1e-12 && (projected[0][0] - 1.0).abs() < 1e-12);
        // a quartic's own values
        let poly: Vec<f32> = dirs
            .iter()
            .map(|w| 0.2 + 0.5 * w.x.powi(4) + 0.3 * w.y * w.y * w.z * w.z)
            .collect();
        let fitted = EvenPoly4::fit(&dirs, &poly);
        for (w, p) in dirs.iter().zip(&poly) {
            assert!((fitted.opacity(*w) - p).abs() < 1e-4);
        }
    }

    #[test]
    fn module_nodes_match_their_voxels() {
        let mut module = VoxelModule::new(3);
        for p in [(17, 18, 19), (30, 16, 16), (20, 31, 25), (5, 5, 5)] {
            module.set(UVec3::new(p.0, p.1, p.2), 2);
        }
        let node = NodeVoxels::from_module(&module, UVec3::splat(16), 16).unwrap();
        assert_eq!(node.voxels(), 3);
        assert!(node.get(UVec3::new(1, 2, 3)) && node.get(UVec3::new(14, 0, 0)));
        assert_eq!(
            node.occupied_box(),
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(15.0, 16.0, 10.0))
        );
        assert!(NodeVoxels::from_module(&module, UVec3::new(32, 0, 0), 16).is_none());
    }
}
