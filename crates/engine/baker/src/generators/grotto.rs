//! Crystal grottos: a hollow rock mound with an entrance tunnel, and crystal clusters growing
//! sparsely out of the chamber's walls, floor, and ceiling.
//!
//! The rock is one module voxelized only as a shell around its outer and inner surfaces. Crystal
//! clusters are a few small, finer-grained modules, each reused by several static pose groups
//! placed at Poisson-spaced wall sites and oriented along the wall normal, so the asset exercises
//! module sharing and rotated groups as well as dense surfaces.

use crate::{
    AssetRecipe,
    generators::{
        crystal::{HUES, PALETTE, Prism, grow_prism},
        palette,
    },
    noise::{Rng, fbm},
    voxelize::{ellipsoid, fill_sdf_shell, tapered_capsule},
};
use glam::{Affine3A, Quat, Vec3};
use serde::{Deserialize, Serialize};
use voxel::{CanonicalAssembly, Motion, PoseGroupTemplate, VoxelModule};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct GrottoParams {
    /// Footprint radius and height of the rock mound, in rock voxels.
    pub radius: f32,
    pub height: f32,
    /// Horizontal radius and vertical semi-axis of the chamber.
    pub chamber_radius: f32,
    pub chamber_height: f32,
    pub tunnel_radius: f32,
    /// Rock shell thickness behind every visible surface.
    pub shell: f32,
    /// Wall sites crystals grow from, and their minimum spacing in rock voxels.
    pub crystal_sites: u32,
    pub site_spacing: f32,
    /// Distinct crystal cluster modules shared between the sites.
    pub crystal_variants: u32,
    /// Crystal voxels per rock voxel edge.
    pub crystal_detail: f32,
}

impl Default for GrottoParams {
    fn default() -> Self {
        Self {
            radius: 230.0,
            height: 190.0,
            chamber_radius: 165.0,
            chamber_height: 105.0,
            tunnel_radius: 48.0,
            shell: 3.0,
            crystal_sites: 13,
            site_spacing: 70.0,
            crystal_variants: 5,
            crystal_detail: 2.0,
        }
    }
}

/// Weathered outer rock and the chamber's own rock, after the shared crystal palette.
const OUTER_ROCK: [u16; 2] = [12, 13];
const CAVE_ROCK: [u16; 3] = [14, 15, 16];
/// Lowest height of the chamber floor above the ground, in rock voxels: above the meadow's
/// grass, so ground tiles under the grotto never poke through.
const FLOOR_HEIGHT: f32 = 13.0;
/// Extent of each crystal cluster module, in crystal voxels.
const CLUSTER_EXTENT: u32 = 192;

pub fn generate(recipe: &AssetRecipe, p: &GrottoParams) -> CanonicalAssembly {
    let mut rng = Rng::new(recipe.seed);
    let seed = recipe.seed as u32;
    let mut rock = VoxelModule::with_extent((2.0 * p.radius + 64.0) as u32);
    let half = rock.extent() as f32 * 0.5;
    let base = Vec3::new(half, 0.0, half);
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];

    const LIPSCHITZ: f32 = 4.0;
    // past this distance from any surface, a value's exact size no longer changes the shell
    // voxelizer's decisions, only its sign does (half a block's diagonal times the bound)
    const FAR: f32 = 48.0;

    // displaces a base distance by a large, slow undulation plus fine surface detail, both
    // gentle enough for the shell voxelizer's block skipping (see `LIPSCHITZ`). Noise is only
    // evaluated where it could matter, which is most of the bake's cost.
    let displace = |d: f32, q: Vec3, scale: f32, salt: u32| {
        let amplitude = scale * 0.35 * 0.5 + 3.5;
        if d.abs() > amplitude + FAR {
            return d;
        }
        d + (fbm(q * 0.012, 3, seed ^ salt) - 0.5) * scale * 0.35
            + (fbm(q * 0.11, 2, seed ^ salt ^ 0x51) - 0.5) * 7.0
    };
    let mound = |q: Vec3| {
        let d = ellipsoid(q, base, axes, Vec3::new(p.radius, p.height, p.radius));
        displace(d, q, p.radius, 0x0A)
    };
    let chamber_center = base + Vec3::Y * p.chamber_height * 0.55;
    let tunnel_start = base + Vec3::new(p.radius * 0.3, p.tunnel_radius * 1.1, 0.0);
    let tunnel_end = base + Vec3::new(p.radius * 1.25, p.tunnel_radius * 0.9, 0.0);
    let cavity = |q: Vec3| {
        let chamber = ellipsoid(
            q,
            chamber_center,
            axes,
            Vec3::new(p.chamber_radius, p.chamber_height, p.chamber_radius * 0.85),
        );
        let tunnel = tapered_capsule(
            q,
            tunnel_start,
            tunnel_end,
            p.tunnel_radius * 1.15,
            p.tunnel_radius,
        );
        let chamber = displace(chamber, q, p.chamber_radius, 0x1B);
        let tunnel = displace(tunnel, q, p.tunnel_radius, 0x2C);
        // a gently uneven floor
        let floor = if q.y > FLOOR_HEIGHT + 7.0 + FAR {
            FLOOR_HEIGHT
        } else {
            FLOOR_HEIGHT + fbm(q * 0.04, 2, seed ^ 0x3E) * 7.0
        };
        chamber.min(tunnel).max(floor - q.y)
    };
    let solid = |q: Vec3| mound(q).max(-cavity(q)).max(-q.y);

    fill_sdf_shell(
        &mut rock,
        base - Vec3::new(p.radius, 0.0, p.radius) * 1.3,
        base + Vec3::new(p.radius * 1.3, p.height * 1.3, p.radius * 1.3),
        solid,
        LIPSCHITZ,
        p.shell,
        |q, d| {
            // the outer surface is the one defining the solid here
            if mound(q) >= d {
                OUTER_ROCK[(fbm(q * 0.05, 2, seed ^ 0x3D) * 2.0).min(1.0) as usize]
            } else {
                // broad tonal variation broken up by fine grain, so no single patch dominates
                let tone =
                    fbm(q * 0.035, 3, seed ^ 0x77) + (fbm(q * 0.4, 1, seed ^ 0x78) - 0.5) * 0.3;
                CAVE_ROCK[((tone - 0.3) * 4.0).clamp(0.0, 2.0) as usize]
            }
        },
    );

    let mut modules = vec![rock];
    let hue_bias = rng.below(HUES.len() as u32) as usize;
    for _ in 0..p.crystal_variants {
        modules.push(crystal_cluster(&mut rng, hue_bias));
    }

    let mut groups = vec![PoseGroupTemplate {
        name: "rock".into(),
        parent: None,
        module: 0,
        rest: Affine3A::from_translation(-base),
        motion: Motion::Static,
    }];
    let cluster_base = Vec3::new(
        CLUSTER_EXTENT as f32 * 0.5,
        0.0,
        CLUSTER_EXTENT as f32 * 0.5,
    );
    let gradient = |q: Vec3| {
        let e = 1.0;
        Vec3::new(
            cavity(q + Vec3::X * e) - cavity(q - Vec3::X * e),
            cavity(q + Vec3::Y * e) - cavity(q - Vec3::Y * e),
            cavity(q + Vec3::Z * e) - cavity(q - Vec3::Z * e),
        )
    };
    let mut sites: Vec<Vec3> = Vec::new();
    for _ in 0..p.crystal_sites * 40 {
        if sites.len() >= p.crystal_sites as usize {
            break;
        }
        let dir = rng.direction();
        let Some(site) = wall_point(&cavity, chamber_center, dir, p.radius * 2.0) else {
            continue;
        };
        // keep the entrance clear, and stay on walls that have rock behind them
        let normal = -gradient(site).normalize_or_zero();
        let near_tunnel =
            tapered_capsule(site, tunnel_start, tunnel_end, 0.0, 0.0) < p.tunnel_radius * 1.8;
        if normal == Vec3::ZERO
            || near_tunnel
            || solid(site - normal * (p.shell + 1.0)) > 0.0
            || sites.iter().any(|s| s.distance(site) < p.site_spacing)
        {
            continue;
        }
        sites.push(site);

        let scale = rng.range(0.7, 1.3) / p.crystal_detail;
        let twist = rng.range(0.0, std::f32::consts::TAU);
        // sink the cluster's base into the wall so it grows out of the rock rather than floating
        let rest = Affine3A::from_translation(site - normal * 5.0)
            * Affine3A::from_quat(Quat::from_rotation_arc(Vec3::Y, normal))
            * Affine3A::from_rotation_y(twist)
            * Affine3A::from_scale(Vec3::splat(scale))
            * Affine3A::from_translation(-cluster_base);
        groups.push(PoseGroupTemplate {
            name: format!("crystals_{}", sites.len()),
            parent: Some(0),
            module: 1 + rng.below(p.crystal_variants) as u16,
            rest,
            motion: Motion::Static,
        });
    }

    let mut colors = PALETTE.to_vec();
    colors.extend([
        ([0.16, 0.17, 0.13], 0.0),
        ([0.22, 0.24, 0.17], 0.0),
        ([0.10, 0.095, 0.09], 0.0),
        ([0.13, 0.12, 0.11], 0.0),
        ([0.165, 0.15, 0.135], 0.0),
    ]);
    CanonicalAssembly {
        name: recipe.name.clone(),
        voxel_size: recipe.voxel_size,
        palette: palette(&colors),
        modules,
        groups,
    }
}

/// Where a ray from `origin` (inside the cavity) first leaves it, found by bisection.
fn wall_point(cavity: &impl Fn(Vec3) -> f32, origin: Vec3, dir: Vec3, far: f32) -> Option<Vec3> {
    // march to the first point outside, then bisect between it and the last point inside
    let step = 4.0;
    let mut inside = 0.0;
    let mut t = step;
    while t < far {
        if cavity(origin + dir * t) > 0.0 {
            let mut outside = t;
            for _ in 0..16 {
                let mid = (inside + outside) * 0.5;
                if cavity(origin + dir * mid) > 0.0 {
                    outside = mid;
                } else {
                    inside = mid;
                }
            }
            return Some(origin + dir * outside);
        }
        inside = t;
        t += step;
    }
    None
}

/// A druse: three to seven stubby prisms rooted in a small patch near the center of the module's
/// base, growing along +Y and fanning outward, with one larger crystal among them.
fn crystal_cluster(rng: &mut Rng, hue_bias: usize) -> VoxelModule {
    let mut module = VoxelModule::with_extent(CLUSTER_EXTENT);
    let center = Vec3::new(
        CLUSTER_EXTENT as f32 * 0.5,
        0.0,
        CLUSTER_EXTENT as f32 * 0.5,
    );
    let hue = HUES[if rng.unit() < 0.6 {
        hue_bias
    } else {
        rng.below(HUES.len() as u32) as usize
    }];
    let count = 3 + rng.below(5);
    for i in 0..count {
        let hero = i == 0;
        let length = if hero {
            rng.range(50.0, 85.0)
        } else {
            rng.range(16.0, 48.0)
        };
        let radius = (length * rng.range(0.17, 0.25)).clamp(3.0, 15.0);
        let offset = if hero {
            Vec3::ZERO
        } else {
            let a = rng.range(0.0, std::f32::consts::TAU);
            Vec3::new(a.cos(), 0.0, a.sin()) * rng.range(6.0, 20.0)
        };
        // smaller crystals lean away from the patch's center
        let lean = if hero {
            rng.direction() * 0.2
        } else {
            offset.normalize_or_zero() * 0.5 + rng.direction() * 0.25
        };
        let dir = (Vec3::Y + lean).normalize();
        grow_prism(
            &mut module,
            Prism {
                // rooted below the base so the module's rest transform buries every root
                base: center + offset - Vec3::Y * 6.0,
                dir,
                length: length + 6.0,
                radius,
                tip: radius * rng.range(1.0, 1.6),
                twist: rng.range(0.0, 1.0),
            },
            hue,
        );
    }
    module
}
