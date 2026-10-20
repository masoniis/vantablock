//! Directional opacity of LOD nodes: measured angular coverage and the compact representations
//! that approximate it (the spec's additional Phase 2 test, section 2.4's SGGX candidate).
//!
//! A filtered hit treats a node as a surface filling its occupied box (see [`crate::lod`]), so the
//! quantity a summary must predict is a node's **angular coverage**: the chance that a ray along
//! `w` that enters the occupied box hits one of the node's voxels. It depends only on the line, so
//! it is symmetric in `w`. [`NodeVoxels::coverage`] measures it with stratified parallel rays.
//! The candidates, all derived from the same voxels:
//!
//! - **Axis proxy** (the spec's): `clamp(sum(|w_a| * c_a), 0, 1)` over the box-relative axis
//!   coverages [`NodeVoxels::axis_coverage`] (the baked [`crate::lod::LodSummary::coverage`]).
//! - **SGGX tensor** ([`Sggx`]): a symmetric positive-semidefinite `S` with
//!   `sigma(w) = sqrt(max(0, w^T S w))`, six coefficients (12 bytes as `f16`). Built from the
//!   node's exposed faces (the usual surface construction, `S = sum(area * n n^T)`), it is diagonal
//!   for voxels, whose faces are axis aligned, and can't tell a leaf along one diagonal from a
//!   leaf along the other, so it is fitted here to measured coverage instead: least squares on
//!   `sigma^2 = coverage^2` over a fixed direction set, then projected onto the PSD cone. The
//!   diagonal tensor `diag(c_a^2)` is the axis coverages combined in `L2` instead of `L1`, the
//!   proxy [`crate::lod::LodSummary::opacity`] evaluates.
//! - **Degree-4 even polynomial** ([`EvenPoly4`]): coverage fitted by the 15 homogeneous quartic
//!   monomials (spherical harmonic bands 0, 2, and 4), the richer reference representation.
//!
//! Projected area is not opacity (spec section 2.4): the fitted tensor's `sigma` is used as
//! coverage only because the fit maps it onto measured coverage.

use crate::{
    lod::{OCCUPIED_STEPS, f16_to_f32, f32_to_f16},
    module::{EMPTY, VoxelModule},
};
use glam::{IVec3, UVec3, Vec3};

/// The occupied voxels of one node, `size` voxels on a side.
#[derive(Clone, Debug)]
pub struct NodeVoxels {
    size: u32,
    bits: Vec<u64>,
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
        let mut node = Self::empty(size);
        let bricks = size.div_ceil(4);
        for bz in 0..bricks {
            for by in 0..bricks {
                for bx in 0..bricks {
                    let local = UVec3::new(bx, by, bz) * 4;
                    let Some(brick) = module.bricks().get(&((corner + local) / 4)) else {
                        continue;
                    };
                    for (k, &material) in brick.materials.iter().enumerate() {
                        let k = k as u32;
                        let p = local + UVec3::new(k & 3, (k >> 2) & 3, k >> 4);
                        if material != EMPTY && p.max_element() < size {
                            node.set(p);
                        }
                    }
                }
            }
        }
        node.finish()
    }

    fn empty(size: u32) -> Self {
        Self {
            size,
            bits: vec![0; (size as usize).pow(3).div_ceil(64)],
            lo: UVec3::splat(size),
            hi: UVec3::ZERO,
        }
    }

    fn set(&mut self, p: UVec3) {
        let i = self.index(p);
        self.bits[i / 64] |= 1 << (i % 64);
        self.lo = self.lo.min(p);
        self.hi = self.hi.max(p + 1);
    }

    fn finish(self) -> Option<Self> {
        (self.lo.x < self.hi.x).then_some(self)
    }

    fn index(&self, p: UVec3) -> usize {
        let s = self.size as usize;
        (p.z as usize * s + p.y as usize) * s + p.x as usize
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn voxels(&self) -> u64 {
        self.bits.iter().map(|w| w.count_ones() as u64).sum()
    }

    pub fn get(&self, p: UVec3) -> bool {
        p.max_element() < self.size && {
            let i = self.index(p);
            self.bits[i / 64] & 1 << (i % 64) != 0
        }
    }

    /// The occupied box in voxels, rounded outward to sixteenths of the node's edge exactly as the
    /// LOD record stores it.
    pub fn occupied_box(&self) -> (Vec3, Vec3) {
        let sixteenth = self.size as f64 / OCCUPIED_STEPS as f64;
        let lo = (self.lo.as_dvec3() / sixteenth).floor() * sixteenth;
        let hi = (self.hi.as_dvec3() / sixteenth).ceil() * sixteenth;
        (lo.as_vec3(), hi.as_vec3())
    }

    /// The fraction of the occupied box's face along each axis covered by the node's projection,
    /// as the bake computes [`crate::lod::LodSummary::coverage`].
    pub fn axis_coverage(&self) -> [f32; 3] {
        let (lo, hi) = self.occupied_box();
        let ext = (hi - lo).as_dvec3();
        let faces = [ext.y * ext.z, ext.x * ext.z, ext.x * ext.y];
        let s = self.size as usize;
        let mut projections = [vec![false; s * s], vec![false; s * s], vec![false; s * s]];
        for z in self.lo.z..self.hi.z {
            for y in self.lo.y..self.hi.y {
                for x in self.lo.x..self.hi.x {
                    if self.get(UVec3::new(x, y, z)) {
                        let (x, y, z) = (x as usize, y as usize, z as usize);
                        projections[0][z * s + y] = true;
                        projections[1][z * s + x] = true;
                        projections[2][y * s + x] = true;
                    }
                }
            }
        }
        std::array::from_fn(|axis| {
            let count = projections[axis].iter().filter(|&&b| b).count() as f64;
            (count / faces[axis]).min(1.0) as f32
        })
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

    /// Whether the segment `[t_enter, t_exit]` of the ray hits a voxel inside the box (a plain
    /// DDA: a measurement, not a traversal that reports identity).
    fn march(&self, origin: Vec3, w: Vec3, t_enter: f32, t_exit: f32, lo: Vec3, hi: Vec3) -> bool {
        let (lo, hi) = (lo.as_ivec3(), hi.as_ivec3());
        let start = origin + w * (t_enter + (t_exit - t_enter) * 1e-4);
        let mut cell = start.floor().as_ivec3().clamp(lo, hi - 1);
        let step = IVec3::new(sign(w.x), sign(w.y), sign(w.z));
        let delta = w.abs().recip();
        let next = |cell: IVec3, axis: usize| {
            if w[axis] == 0.0 {
                f32::INFINITY
            } else {
                let plane = cell[axis] + (step[axis] > 0) as i32;
                (plane as f32 - origin[axis]) / w[axis]
            }
        };
        let mut t_next = Vec3::new(next(cell, 0), next(cell, 1), next(cell, 2));
        loop {
            if self.get(cell.as_uvec3()) {
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
        }
    }
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

/// The spec's `L1` axis-coverage opacity proxy (the renderer now uses the `L2` combination,
/// [`Sggx::from_axis_coverage`]).
pub fn axis_opacity(coverage: [f32; 3], dir: Vec3) -> f32 {
    dir.normalize_or_zero()
        .abs()
        .dot(Vec3::from(coverage))
        .clamp(0.0, 1.0)
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
