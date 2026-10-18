//! Branching trees: a static trunk and crown, plus boughs that are separate hinged pose groups
//! so wind can move each one independently.

use crate::{
    AssetRecipe,
    generators::palette,
    noise::{Rng, fbm},
    voxelize::{capsule_bounds, ellipsoid, fill_sdf, frame_from, tapered_capsule},
};
use glam::{Affine3A, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};
use voxel::{CanonicalAssembly, Motion, PoseGroupTemplate, VoxelModule};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct TreeParams {
    /// Trunk height in voxels.
    pub height: f32,
    pub trunk_radius: f32,
    /// Number of articulated boughs.
    pub boughs: u32,
    pub bough_length: f32,
    /// Branching generations below each bough.
    pub branch_depth: u32,
    pub leaf_size: f32,
    pub leaves_per_twig: u32,
    /// Maximum bough hinge angle in radians.
    pub sway: f32,
}

impl Default for TreeParams {
    fn default() -> Self {
        Self {
            height: 440.0,
            trunk_radius: 20.0,
            boughs: 7,
            bough_length: 190.0,
            branch_depth: 3,
            leaf_size: 6.0,
            leaves_per_twig: 9,
            sway: 0.07,
        }
    }
}

const BARK_DARK: u16 = 1;
const BARK_MID: u16 = 2;
const BARK_GRAY: u16 = 3;
const MOSS: u16 = 4;
const LEAVES: [u16; 4] = [5, 6, 7, 8];

/// Where boughs start in their own module, leaving room to grow along +x.
const BOUGH_ANCHOR: Vec3 = Vec3::new(10.0, 104.0, 128.0);
const BARK_SHELL: f32 = 4.0;

pub fn generate(recipe: &AssetRecipe, p: &TreeParams) -> CanonicalAssembly {
    let mut rng = Rng::new(recipe.seed);
    let seed = recipe.seed as u32;

    let mut trunk = VoxelModule::with_extent((p.height * 1.15) as u32 + 2 * p.trunk_radius as u32);
    let extent = trunk.extent() as f32;
    let base = Vec3::new(extent * 0.5, 2.0, extent * 0.5);

    // trunk centerline with a gentle wander, tapering toward the crown
    let segments = 28;
    let centerline: Vec<(Vec3, f32)> = (0..=segments)
        .map(|i| {
            let f = i as f32 / segments as f32;
            let wander = Vec3::new(
                fbm(Vec3::new(f * 3.0, 0.0, 1.7), 3, seed) - 0.5,
                0.0,
                fbm(Vec3::new(f * 3.0, 4.1, 0.0), 3, seed) - 0.5,
            ) * p.trunk_radius
                * 2.4
                * f;
            let flare = p.trunk_radius * 0.9 * (-f * p.height / (p.trunk_radius * 0.8)).exp();
            let radius = p.trunk_radius * (1.0 - 0.62 * f.powf(0.8)) + flare;
            (base + Vec3::Y * p.height * f + wander, radius)
        })
        .collect();

    for pair in centerline.windows(2) {
        let ((a, ra), (b, rb)) = (pair[0], pair[1]);
        bark_capsule(&mut trunk, a, b, ra, rb, true, seed);
    }

    // buttress roots spreading along the ground
    for i in 0..5 {
        let angle = i as f32 / 5.0 * TAU + rng.range(-0.3, 0.3);
        let dir = Vec3::new(angle.cos(), -0.08, angle.sin());
        let start = base + Vec3::Y * p.trunk_radius * 0.6;
        let end = start + dir * p.trunk_radius * rng.range(2.2, 3.2);
        bark_capsule(
            &mut trunk,
            start,
            end,
            p.trunk_radius * 0.55,
            2.5,
            false,
            seed,
        );
    }

    // a static crown at the top keeps the silhouette full
    let (top, top_radius) = *centerline.last().unwrap();
    for i in 0..4 {
        let angle = i as f32 / 4.0 * TAU + rng.range(0.0, 1.0);
        let dir = (Vec3::new(angle.cos(), 1.6, angle.sin())).normalize();
        grow_branch(
            &mut trunk,
            &mut rng,
            p,
            seed,
            top,
            dir,
            p.bough_length * 0.45,
            top_radius * 0.7,
            p.branch_depth,
        );
    }

    let mut modules = vec![trunk];
    let mut groups = vec![PoseGroupTemplate {
        name: "trunk".into(),
        parent: None,
        module: 0,
        // the trunk base is the asset's ground anchor
        rest: Affine3A::from_translation(-base),
        motion: Motion::Static,
    }];

    let golden = PI * (3.0 - 5f32.sqrt());
    for i in 0..p.boughs {
        let f = if p.boughs > 1 {
            i as f32 / (p.boughs - 1) as f32
        } else {
            0.5
        };
        let attach_height = p.height * (0.4 + 0.42 * f) + rng.range(-8.0, 8.0);
        let azimuth = i as f32 * golden + rng.range(-0.25, 0.25);

        // the trunk point at this height, nudged into the bark so the joint looks grown
        let (trunk_point, trunk_radius) =
            sample_centerline(&centerline, (attach_height - base.y) / p.height);
        let outward = Vec3::new(azimuth.cos(), 0.0, azimuth.sin());
        let attach = trunk_point + outward * trunk_radius * 0.4;

        let mut bough = VoxelModule::new(4);
        let tilt = rng.range(0.25, 0.6);
        let length = p.bough_length * (1.0 - 0.35 * f) * rng.range(0.85, 1.1);
        let radius = trunk_radius * 0.55;
        let dir = Vec3::new(tilt.cos(), tilt.sin(), 0.0);
        grow_branch(
            &mut bough,
            &mut rng,
            p,
            seed.wrapping_add(i + 1),
            BOUGH_ANCHOR,
            dir,
            length,
            radius,
            p.branch_depth,
        );
        modules.push(bough);

        // bough-local +x points outward from the trunk along the azimuth
        let rest = Affine3A::from_translation(attach)
            * Affine3A::from_quat(Quat::from_rotation_y(-azimuth))
            * Affine3A::from_translation(-BOUGH_ANCHOR);
        let sway = p.sway * rng.range(0.8, 1.3);
        groups.push(PoseGroupTemplate {
            name: format!("bough_{i}"),
            parent: Some(0),
            module: i as u16 + 1,
            rest,
            motion: Motion::Hinge {
                pivot: BOUGH_ANCHOR,
                axis: Vec3::Z,
                min_angle: -sway,
                max_angle: sway,
                // longer boughs respond slower
                response_hz: rng.range(0.3, 0.45) * (1.3 - 0.5 * length / p.bough_length),
            },
        });
    }

    CanonicalAssembly {
        name: recipe.name.clone(),
        voxel_size: recipe.voxel_size,
        palette: palette(&[
            ([0.10, 0.06, 0.035], 0.0),
            ([0.17, 0.10, 0.055], 0.0),
            ([0.20, 0.17, 0.14], 0.0),
            ([0.14, 0.20, 0.06], 0.0),
            ([0.05, 0.16, 0.03], 0.0),
            ([0.10, 0.28, 0.05], 0.0),
            ([0.24, 0.36, 0.06], 0.0),
            ([0.55, 0.32, 0.05], 0.0),
        ]),
        modules,
        groups,
    }
}

fn sample_centerline(centerline: &[(Vec3, f32)], f: f32) -> (Vec3, f32) {
    let x = f.clamp(0.0, 1.0) * (centerline.len() - 1) as f32;
    let i = (x as usize).min(centerline.len() - 2);
    let t = x - i as f32;
    let (a, ra) = centerline[i];
    let (b, rb) = centerline[i + 1];
    (a.lerp(b, t), ra + (rb - ra) * t)
}

/// Writes a bark-textured capsule. Thick parts only keep a shell since their interior is never
/// visible.
fn bark_capsule(
    module: &mut VoxelModule,
    a: Vec3,
    b: Vec3,
    ra: f32,
    rb: f32,
    shell: bool,
    seed: u32,
) {
    let (min, max) = capsule_bounds(a, b, ra.max(rb));
    let axis = (b - a).normalize_or_zero();
    fill_sdf(
        module,
        min,
        max,
        |q| tapered_capsule(q, a, b, ra, rb),
        |q, d| {
            if shell && d < -BARK_SHELL {
                return 0;
            }
            bark_material(q, a, axis, seed)
        },
    );
}

fn bark_material(q: Vec3, a: Vec3, axis: Vec3, seed: u32) -> u16 {
    let radial = (q - a) - axis * (q - a).dot(axis);
    let angle = radial.z.atan2(radial.x);
    let along = (q - a).dot(axis);
    // vertical ridges broken up by noise
    let ridge = (angle * 11.0 + fbm(q * 0.05, 3, seed) * 7.0 + along * 0.015).sin();
    if fbm(q * 0.03 + Vec3::splat(9.0), 2, seed) > 0.66 && radial.y > -0.2 {
        MOSS
    } else if ridge > 0.55 {
        BARK_DARK
    } else if fbm(q * 0.11, 2, seed ^ 0x55) > 0.55 {
        BARK_GRAY
    } else {
        BARK_MID
    }
}

/// Recursively grows a branch with child branches and leafy twigs.
#[allow(clippy::too_many_arguments)]
fn grow_branch(
    module: &mut VoxelModule,
    rng: &mut Rng,
    p: &TreeParams,
    seed: u32,
    start: Vec3,
    dir: Vec3,
    length: f32,
    radius: f32,
    depth: u32,
) {
    let segments = ((length / 14.0) as u32).max(3);
    let seg_len = length / segments as f32;
    let mut pos = start;
    let mut dir = dir.normalize();
    let end_radius = (radius * 0.45).max(1.0);

    for s in 0..segments {
        let f0 = s as f32 / segments as f32;
        let f1 = (s + 1) as f32 / segments as f32;
        // wander, with young wood reaching up and thin wood drooping
        let tropism = if depth >= 2 {
            Vec3::Y * 0.08
        } else {
            Vec3::NEG_Y * 0.05
        };
        dir = (dir + rng.direction() * 0.18 + tropism).normalize();
        let next = pos + dir * seg_len;
        let (r0, r1) = (
            radius + (end_radius - radius) * f0,
            radius + (end_radius - radius) * f1,
        );
        bark_capsule(module, pos, next, r0, r1, false, seed);

        if depth > 0 && s > 0 && rng.unit() < 0.55 {
            let frame = frame_from(dir);
            let around = rng.range(0.0, TAU);
            let spread = rng.range(0.5, 1.0);
            let child_dir = (dir * spread.cos()
                + (frame[0] * around.cos() + frame[1] * around.sin()) * spread.sin())
            .normalize();
            let child_length = length * rng.range(0.4, 0.6) * (1.6 - f1);
            grow_branch(
                module,
                rng,
                p,
                seed,
                next,
                child_dir,
                child_length,
                r1 * 0.7,
                depth - 1,
            );
        }
        pos = next;
    }

    if depth == 0 || radius < 2.0 {
        add_leaves(module, rng, p, start, pos);
    } else {
        // finish every limb with a leafy tuft
        add_leaves(module, rng, p, pos - dir * seg_len * 2.0, pos);
    }
}

/// Scatters flat, oriented leaves along the outer part of a twig.
fn add_leaves(module: &mut VoxelModule, rng: &mut Rng, p: &TreeParams, from: Vec3, to: Vec3) {
    for _ in 0..p.leaves_per_twig {
        let along = from.lerp(to, rng.range(0.3, 1.0));
        let center = along + rng.direction() * p.leaf_size * 1.1;
        // leaves mostly face up toward the sky
        let normal = (rng.direction() + Vec3::Y * 1.2).normalize();
        let mut frame = frame_from(normal);
        let spin = rng.range(0.0, TAU);
        let (u, v) = (frame[0], frame[1]);
        frame[0] = u * spin.cos() + v * spin.sin();
        frame[1] = v * spin.cos() - u * spin.sin();
        let size = p.leaf_size * rng.range(0.75, 1.25);
        let radii = Vec3::new(size, size * 0.55, 0.6);
        let material = LEAVES[if rng.unit() < 0.04 {
            3
        } else {
            rng.below(3) as usize
        }];
        let reach = Vec3::splat(size + 1.0);
        fill_sdf(
            module,
            center - reach,
            center + reach,
            |q| ellipsoid(q, center, [frame[0], frame[1], frame[2]], radii),
            |_, _| material,
        );
    }
}
