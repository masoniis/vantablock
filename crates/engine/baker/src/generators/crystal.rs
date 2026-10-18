//! Crystal clusters: faceted hexagonal prisms with pointed tips growing out of a rock base.

use crate::{
    AssetRecipe,
    generators::palette,
    noise::{Rng, fbm},
    voxelize::{ellipsoid, fill_sdf, frame_from, hexagon},
};
use glam::{Affine3A, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use voxel::{CanonicalAssembly, Motion, PoseGroupTemplate, VoxelModule};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CrystalParams {
    /// Module extent in voxels.
    pub extent: u32,
    pub crystals: u32,
    pub max_length: f32,
    pub max_radius: f32,
    pub rock_radius: f32,
}

impl Default for CrystalParams {
    fn default() -> Self {
        Self {
            extent: 256,
            crystals: 18,
            max_length: 150.0,
            max_radius: 17.0,
            rock_radius: 70.0,
        }
    }
}

pub(crate) const ROCK: [u16; 3] = [1, 2, 3];
/// Each hue has a body material and a brighter, glowing tip.
pub(crate) const HUES: [(u16, u16); 4] = [(4, 5), (6, 7), (8, 9), (10, 11)];
const ROCK_SHELL: f32 = 4.0;

/// Dark rock followed by the crystal hues, in the order [`ROCK`] and [`HUES`] index.
pub(crate) const PALETTE: [([f32; 3], f32); 11] = [
    ([0.09, 0.085, 0.08], 0.0),
    ([0.14, 0.13, 0.12], 0.0),
    ([0.05, 0.05, 0.055], 0.0),
    ([0.15, 0.62, 0.75], 0.0),
    ([0.45, 0.95, 1.0], 0.9),
    ([0.40, 0.20, 0.80], 0.0),
    ([0.75, 0.55, 1.0], 0.9),
    ([0.80, 0.18, 0.50], 0.0),
    ([1.0, 0.50, 0.75], 0.9),
    ([0.85, 0.55, 0.15], 0.0),
    ([1.0, 0.82, 0.45], 0.9),
];

/// One faceted hexagonal prism with a pointed tip.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Prism {
    pub base: Vec3,
    pub dir: Vec3,
    pub length: f32,
    pub radius: f32,
    /// Length of the pointed tip.
    pub tip: f32,
    /// Rotation of the facets about the prism's axis.
    pub twist: f32,
}

/// Voxelizes a prism with `body` material, and `glow` near its tip.
pub(crate) fn grow_prism(module: &mut VoxelModule, prism: Prism, (body, glow): (u16, u16)) {
    let Prism {
        base,
        dir,
        length,
        radius,
        tip,
        twist,
    } = prism;
    let frame = frame_from(dir);
    let (u, v) = (
        frame[0] * twist.cos() + frame[1] * twist.sin(),
        frame[1] * twist.cos() - frame[0] * twist.sin(),
    );

    let crystal_sdf = |q: Vec3| {
        let d = q - base;
        let h = d.dot(dir);
        let r = if h > length - tip {
            radius * ((length - h) / tip).max(0.0)
        } else {
            radius
        };
        hexagon(Vec2::new(d.dot(u), d.dot(v)), r)
            .max(-h)
            .max(h - length)
    };
    let reach = Vec3::splat(radius + 2.0);
    let end = base + dir * length;
    fill_sdf(
        module,
        base.min(end) - reach,
        base.max(end) + reach,
        crystal_sdf,
        |q, _| {
            if (q - base).dot(dir) > length - tip * 0.6 {
                glow
            } else {
                body
            }
        },
    );
}

pub fn generate(recipe: &AssetRecipe, p: &CrystalParams) -> CanonicalAssembly {
    let mut rng = Rng::new(recipe.seed);
    let seed = recipe.seed as u32;
    let mut module = VoxelModule::with_extent(p.extent);
    let extent = module.extent() as f32;

    let rock_center = Vec3::new(extent * 0.5, p.rock_radius * 0.4, extent * 0.5);
    let rock_frame = [Vec3::X, Vec3::Y, Vec3::Z];
    let rock_radii = Vec3::new(p.rock_radius, p.rock_radius * 0.5, p.rock_radius * 0.85);
    let rock_sdf = |q: Vec3| {
        ellipsoid(q, rock_center, rock_frame, rock_radii)
            + (fbm(q * 0.045, 4, seed) - 0.5) * p.rock_radius * 0.55
    };
    fill_sdf(
        &mut module,
        rock_center - rock_radii * 1.4,
        rock_center + rock_radii * 1.4,
        rock_sdf,
        |q, d| {
            if d < -ROCK_SHELL {
                return 0;
            }
            ROCK[(fbm(q * 0.09, 2, seed ^ 0x77) * 3.0).min(2.0) as usize]
        },
    );

    // a few large hero crystals, then a scatter of small ones
    let hue_bias = rng.below(HUES.len() as u32) as usize;
    for i in 0..p.crystals {
        let hero = i < p.crystals / 3;
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let spread = rng.range(0.0, if hero { 0.45 } else { 0.85 });
        let base = rock_center
            + Vec3::new(
                angle.cos() * spread * rock_radii.x,
                rock_radii.y * 0.6,
                angle.sin() * spread * rock_radii.z,
            );
        let dir = (Vec3::Y * 1.4 + rng.direction() * if hero { 0.6 } else { 1.0 }).normalize();
        let length = p.max_length
            * if hero {
                rng.range(0.65, 1.0)
            } else {
                rng.range(0.2, 0.45)
            };
        let radius = (length * rng.range(0.1, 0.15)).min(p.max_radius).max(3.0);
        let tip = radius * rng.range(1.3, 2.2);
        let hue = HUES[if rng.unit() < 0.7 {
            hue_bias
        } else {
            rng.below(HUES.len() as u32) as usize
        }];
        let twist = rng.range(0.0, 1.0);
        grow_prism(
            &mut module,
            Prism {
                base,
                dir,
                length,
                radius,
                tip,
                twist,
            },
            hue,
        );
    }

    CanonicalAssembly {
        name: recipe.name.clone(),
        voxel_size: recipe.voxel_size,
        palette: palette(&PALETTE),
        modules: vec![module],
        groups: vec![PoseGroupTemplate {
            name: "cluster".into(),
            parent: None,
            module: 0,
            rest: Affine3A::from_translation(-Vec3::new(extent * 0.5, 0.0, extent * 0.5)),
            motion: Motion::Static,
        }],
    }
}
