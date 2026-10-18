//! Rays, bounding boxes, and the slab test shared by every level of the traversal.
//!
//! **Ray convention:** world rays use a normalized direction. Transforming a ray into an
//! instance or pose-group frame applies the full affine inverse to both origin and direction
//! *without renormalizing*, so a local hit parameter `t` is the same distance along the original
//! world ray. This keeps nearest-hit ordering across frames a plain comparison of `t`.

use glam::{Affine3A, Vec3};

/// Smallest magnitude a ray direction component may have.
///
/// Zero components are replaced with this value (keeping their sign) so the slab test never
/// multiplies `0 * inf`. The shaders apply the identical rule, which keeps CPU and GPU results
/// comparable.
pub const MIN_DIRECTION_COMPONENT: f32 = 1e-8;

/// A ray with a precomputed reciprocal direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
    pub inv_dir: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        let dir = Vec3::new(
            sanitize_component(dir.x),
            sanitize_component(dir.y),
            sanitize_component(dir.z),
        );
        Self {
            origin,
            dir,
            inv_dir: dir.recip(),
        }
    }

    /// The point at parameter `t` along the ray.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }

    /// Transforms the ray by an affine map, preserving the meaning of `t` (see module docs).
    pub fn transformed(&self, transform: &Affine3A) -> Self {
        Self::new(
            transform.transform_point3(self.origin),
            transform.transform_vector3(self.dir),
        )
    }
}

fn sanitize_component(c: f32) -> f32 {
    if c.abs() >= MIN_DIRECTION_COMPONENT {
        c
    } else if c < 0.0 {
        -MIN_DIRECTION_COMPONENT
    } else {
        MIN_DIRECTION_COMPONENT
    }
}

/// An axis-aligned bounding box. An empty box has `min > max` on every axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// The bounding box of a sphere.
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self::new(center - Vec3::splat(radius), center + Vec3::splat(radius))
    }

    pub fn is_empty(&self) -> bool {
        self.min.cmpgt(self.max).any()
    }

    pub fn union(&self, other: &Self) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extent(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    /// Surface area, used for BVH quality metrics.
    pub fn surface_area(&self) -> f32 {
        if self.is_empty() {
            return 0.0;
        }
        let e = self.extent();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }

    pub fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(a.x, b.y, b.z),
            Vec3::new(b.x, b.y, b.z),
        ]
    }

    pub fn contains_point(&self, p: Vec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    pub fn contains_aabb(&self, other: &Self) -> bool {
        other.is_empty() || (other.min.cmpge(self.min).all() && other.max.cmple(self.max).all())
    }

    /// Grows the box by a relative and absolute margin to absorb floating point error when a
    /// bound must stay conservative.
    pub fn padded(&self) -> Self {
        if self.is_empty() {
            return *self;
        }
        let pad = self.extent() * 1e-4 + Vec3::splat(1e-4);
        Self::new(self.min - pad, self.max + pad)
    }

    /// A conservative box around this box after an affine transform (Arvo's method).
    pub fn transformed(&self, transform: &Affine3A) -> Self {
        if self.is_empty() {
            return *self;
        }
        let center = transform.transform_point3(self.center());
        let half = self.extent() * 0.5;
        let m = transform.matrix3;
        let abs_half = m.x_axis.abs() * half.x + m.y_axis.abs() * half.y + m.z_axis.abs() * half.z;
        Self::new(center - Vec3::from(abs_half), center + Vec3::from(abs_half)).padded()
    }

    /// The slab test: the parametric interval `(t_enter, t_exit)` where the ray is inside the
    /// box. The ray misses when `t_enter > t_exit`. Every hierarchy level uses this same
    /// formula, so a voxel inside a node never reports an interval outside its node's interval.
    pub fn ray_interval(&self, ray: &Ray) -> (f32, f32) {
        slab(self.min, self.max, ray)
    }
}

/// See [`Aabb::ray_interval`].
#[inline]
pub fn slab(min: Vec3, max: Vec3, ray: &Ray) -> (f32, f32) {
    let t0 = (min - ray.origin) * ray.inv_dir;
    let t1 = (max - ray.origin) * ray.inv_dir;
    let near = t0.min(t1);
    let far = t0.max(t1);
    (near.max_element(), far.min_element())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    #[test]
    fn zero_direction_components_are_nudged() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, -0.0, 1.0));
        assert_eq!(ray.dir.x, MIN_DIRECTION_COMPONENT);
        // -0.0 is not less than zero, matching the shader's `select`
        assert_eq!(ray.dir.y, MIN_DIRECTION_COMPONENT);
        assert!(ray.inv_dir.is_finite());
    }

    #[test]
    fn transformed_rays_preserve_t() {
        let world = Ray::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.3, -0.5, 0.8).normalize(),
        );
        let local_from_world = Affine3A::from_scale_rotation_translation(
            Vec3::splat(32.0),
            Quat::from_rotation_y(0.7),
            Vec3::new(-5.0, 1.0, 9.0),
        );
        let local = world.transformed(&local_from_world);
        for t in [0.0, 1.5, 10.0] {
            let expected = local_from_world.transform_point3(world.at(t));
            assert!(local.at(t).distance(expected) < 1e-3);
        }
    }

    #[test]
    fn transformed_bounds_contain_transformed_corners() {
        let b = Aabb::new(Vec3::new(-1.0, 0.0, 2.0), Vec3::new(3.0, 5.0, 4.0));
        let m = Affine3A::from_rotation_translation(
            Quat::from_euler(glam::EulerRot::XYZ, 0.3, 1.1, -0.4),
            Vec3::new(10.0, -3.0, 0.5),
        );
        let t = b.transformed(&m);
        for c in b.corners() {
            assert!(t.contains_point(m.transform_point3(c)));
        }
    }

    #[test]
    fn slab_hits_and_misses() {
        let b = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let hit = Ray::new(Vec3::new(0.5, 0.5, -1.0), Vec3::Z);
        let (t0, t1) = b.ray_interval(&hit);
        assert!((t0 - 1.0).abs() < 1e-6 && (t1 - 2.0).abs() < 1e-6);

        let miss = Ray::new(Vec3::new(2.0, 0.5, -1.0), Vec3::Z);
        let (t0, t1) = b.ray_interval(&miss);
        assert!(t0 > t1);
    }
}
