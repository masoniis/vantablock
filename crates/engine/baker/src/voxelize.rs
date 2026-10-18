//! Signed distance primitives and SDF voxelization.

use glam::{Vec2, Vec3};
use voxel::{MaterialId, VoxelModule};

/// Writes every voxel in `[min, max)` whose center has `sdf <= 0`. `material` picks the voxel's
/// material from its center and distance; returning `0` leaves the voxel untouched.
pub fn fill_sdf(
    module: &mut VoxelModule,
    min: Vec3,
    max: Vec3,
    sdf: impl Fn(Vec3) -> f32,
    mut material: impl FnMut(Vec3, f32) -> MaterialId,
) {
    let extent = module.extent() as f32;
    let lo = min.floor().max(Vec3::ZERO);
    let hi = max.ceil().min(Vec3::splat(extent));
    for z in lo.z as u32..hi.z as u32 {
        for y in lo.y as u32..hi.y as u32 {
            for x in lo.x as u32..hi.x as u32 {
                let center = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                let d = sdf(center);
                if d <= 0.0 {
                    let m = material(center, d);
                    if m != 0 {
                        module.set(glam::UVec3::new(x, y, z), m);
                    }
                }
            }
        }
    }
}

/// Like [`fill_sdf`], but only writes the solid within `shell` voxels of the surface, skipping
/// empty space and buried interior by recursively subdividing `16^3` blocks: a block whose
/// center is farther from the shell than the sdf can change across it is skipped whole. That
/// makes large hollow shapes cheap.
///
/// `lipschitz` bounds how fast `sdf` can change per voxel; skipping trusts it, so noise
/// displacements must be accounted for or thin features near block corners can be lost.
pub fn fill_sdf_shell(
    module: &mut VoxelModule,
    min: Vec3,
    max: Vec3,
    sdf: impl Fn(Vec3) -> f32,
    lipschitz: f32,
    shell: f32,
    mut material: impl FnMut(Vec3, f32) -> MaterialId,
) {
    const BLOCK: u32 = 16;
    let extent = module.extent() as f32;
    let lo = min.floor().max(Vec3::ZERO).as_uvec3();
    let hi = max.ceil().min(Vec3::splat(extent)).as_uvec3();
    let mut shell_block = ShellBlock {
        module,
        sdf: &sdf,
        material: &mut material,
        lipschitz,
        shell,
        lo,
        hi,
    };
    for z in (lo.z..hi.z).step_by(BLOCK as usize) {
        for y in (lo.y..hi.y).step_by(BLOCK as usize) {
            for x in (lo.x..hi.x).step_by(BLOCK as usize) {
                shell_block.fill(glam::UVec3::new(x, y, z), BLOCK);
            }
        }
    }
}

struct ShellBlock<'a, S, M> {
    module: &'a mut VoxelModule,
    sdf: &'a S,
    material: &'a mut M,
    lipschitz: f32,
    shell: f32,
    lo: glam::UVec3,
    hi: glam::UVec3,
}

impl<S: Fn(Vec3) -> f32, M: FnMut(Vec3, f32) -> MaterialId> ShellBlock<'_, S, M> {
    fn fill(&mut self, corner: glam::UVec3, size: u32) {
        if corner.cmpge(self.hi).any() {
            return;
        }
        let center = corner.as_vec3() + size as f32 * 0.5;
        let d = (self.sdf)(center);
        if size == 1 {
            if d <= 0.0 && d > -self.shell && corner.cmpge(self.lo).all() {
                let m = (self.material)(center, d);
                if m != 0 {
                    self.module.set(corner, m);
                }
            }
            return;
        }
        // how far the sdf can differ from the center anywhere in the block, voxel centers
        // being at most half a voxel inside its faces
        let reach = (size as f32 * 0.5 * 3f32.sqrt()) * self.lipschitz;
        if d > reach || d < -self.shell - reach {
            return;
        }
        let half = size / 2;
        for k in 0..8 {
            let offset = glam::UVec3::new(k & 1, (k >> 1) & 1, k >> 2) * half;
            self.fill(corner + offset, half);
        }
    }
}

/// A capsule whose radius tapers linearly from `ra` at `a` to `rb` at `b`.
pub fn tapered_capsule(p: Vec3, a: Vec3, b: Vec3, ra: f32, rb: f32) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t) - (ra + (rb - ra) * t)
}

/// The bounding box of a tapered capsule.
pub fn capsule_bounds(a: Vec3, b: Vec3, r: f32) -> (Vec3, Vec3) {
    (
        a.min(b) - Vec3::splat(r + 1.0),
        a.max(b) + Vec3::splat(r + 1.0),
    )
}

/// An ellipsoid with semi-axes `radii` along the orthonormal frame `(u, v, w)` (approximate
/// distance, exact sign).
pub fn ellipsoid(p: Vec3, center: Vec3, frame: [Vec3; 3], radii: Vec3) -> f32 {
    let d = p - center;
    let q = Vec3::new(d.dot(frame[0]), d.dot(frame[1]), d.dot(frame[2])) / radii;
    (q.length() - 1.0) * radii.min_element()
}

/// A regular hexagon of circumradius-ish size `r` (iq's exact 2D hexagon distance).
pub fn hexagon(p: Vec2, r: f32) -> f32 {
    let k = Vec3::new(-0.866_025_4, 0.5, 0.577_350_26);
    let mut p = p.abs();
    p -= 2.0 * Vec2::new(k.x, k.y).dot(p).min(0.0) * Vec2::new(k.x, k.y);
    p -= Vec2::new(p.x.clamp(-k.z * r, k.z * r), r);
    p.length() * p.y.signum()
}

/// Any orthonormal frame whose third axis is `w`.
pub fn frame_from(w: Vec3) -> [Vec3; 3] {
    let w = w.normalize();
    let helper = if w.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = helper.cross(w).normalize();
    [u, w.cross(u), w]
}
