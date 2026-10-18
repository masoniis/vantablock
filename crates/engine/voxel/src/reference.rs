//! A brute-force intersection reference and the rules every traversal is judged by.
//!
//! The reference slab-tests every occupied voxel, independent of any hierarchy. A traversal hit
//! is accepted when:
//!
//! - it names an occupied voxel the ray genuinely reaches, at that voxel's entry distance;
//! - nothing with a positive-length intersection lies meaningfully closer.
//!
//! Exact float ties (rays through voxel edges or corners) touch several voxels at the same
//! distance. Any of them is a correct identity; what is never acceptable is a cell the ray does
//! not touch or a hit behind solid geometry. Both the CPU 64-tree tests and the GPU conformance
//! tests use [`check_hit`].
//!
//! Intersections shorter than [`contact_epsilon`] count as grazing contacts that may be skipped.
//! The threshold scales with coordinate magnitude because f32 positions far from the origin
//! cannot resolve slivers smaller than a few ulps; no f32 traversal can do better.

use crate::{
    math::{Ray, slab},
    module::{EMPTY, VoxelModule},
};
use glam::{UVec3, Vec3};

/// The length (in voxels) below which an intersection entered at `t` counts as a grazing contact.
pub fn contact_epsilon(ray: &Ray, t: f32) -> f32 {
    let magnitude = ray
        .origin
        .abs()
        .max_element()
        .max(ray.at(t).abs().max_element());
    1e-4 + 16.0 * f32::EPSILON * magnitude
}

/// Distance tolerance when comparing hit parameters, relative to `max(1, t)`.
pub const T_TOLERANCE: f32 = 2e-4;

/// The nearest voxel entry among intersections of positive length, or `None` if the ray only
/// grazes (or misses) the module within `[t_min, t_max]`.
pub fn nearest_solid_entry(module: &VoxelModule, ray: &Ray, t_min: f32, t_max: f32) -> Option<f32> {
    module
        .voxels()
        .filter_map(|(p, _)| {
            let (te, tx) = voxel_interval(p, ray);
            let te = te.max(t_min);
            let tx = tx.min(t_max);
            (tx - te > contact_epsilon(ray, te)).then_some(te)
        })
        .min_by(f32::total_cmp)
}

fn voxel_interval(p: UVec3, ray: &Ray) -> (f32, f32) {
    slab(p.as_vec3(), p.as_vec3() + Vec3::ONE, ray)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= T_TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

/// Checks a traversal's answer (`(t, cell)` or a miss) against the reference rules above.
pub fn check_hit(
    module: &VoxelModule,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
    hit: Option<(f32, UVec3)>,
) -> Result<(), String> {
    let solid = nearest_solid_entry(module, ray, t_min, t_max);

    let Some((t, cell)) = hit else {
        return match solid {
            Some(expected) => Err(format!("missed solid geometry at t = {expected}")),
            None => Ok(()),
        };
    };

    if !module.in_bounds(cell) || module.get(cell) == EMPTY {
        return Err(format!("hit cell {cell} is empty"));
    }
    let (te, tx) = voxel_interval(cell, ray);
    let entry = te.max(t_min);
    if !close(t, entry) {
        return Err(format!("hit t = {t} but cell {cell} is entered at {entry}"));
    }
    if entry > tx.min(t_max) + T_TOLERANCE * tx.abs().max(1.0) {
        return Err(format!(
            "ray never reaches cell {cell} (interval {te}..{tx})"
        ));
    }
    if let Some(expected) = solid
        && t > expected
        && !close(t, expected)
    {
        return Err(format!(
            "hit cell {cell} at t = {t} behind solid geometry at t = {expected}"
        ));
    }
    Ok(())
}
