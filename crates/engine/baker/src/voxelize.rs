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
