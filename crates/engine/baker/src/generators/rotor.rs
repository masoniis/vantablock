//! A kinetic sculpture: the spec's minimal articulated test asset. Two pose groups share one
//! immutable blade module while spinning about different axes at different rates.

use crate::{
    AssetRecipe,
    generators::palette,
    voxelize::{capsule_bounds, fill_sdf, tapered_capsule},
};
use glam::{Affine3A, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};
use voxel::{CanonicalAssembly, Motion, PoseGroupTemplate, VoxelModule};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct RotorParams {
    pub mast_height: f32,
    pub blade_length: f32,
    pub blades: u32,
    /// Spin rates of the two rotors, in radians per second.
    pub rate_a: f32,
    pub rate_b: f32,
}

impl Default for RotorParams {
    fn default() -> Self {
        Self {
            mast_height: 170.0,
            blade_length: 100.0,
            blades: 3,
            rate_a: 1.1,
            rate_b: -0.7,
        }
    }
}

const STONE: u16 = 1;
const BRASS: u16 = 2;
const BLADE: u16 = 3;
const BLADE_TIP: u16 = 4;
/// The hub sits at the center of the blade module and is the rotation pivot.
const HUB: Vec3 = Vec3::new(128.0, 128.0, 128.0);

pub fn generate(recipe: &AssetRecipe, p: &RotorParams) -> CanonicalAssembly {
    // pedestal, mast, and a side arm carrying the second rotor
    let mut base = VoxelModule::new(4);
    let center = Vec3::new(128.0, 0.0, 128.0);
    let solid = |m: &mut VoxelModule, a: Vec3, b: Vec3, r: f32, material: u16| {
        let (min, max) = capsule_bounds(a, b, r);
        fill_sdf(
            m,
            min,
            max,
            |q| tapered_capsule(q, a, b, r, r),
            |_, _| material,
        );
    };
    solid(
        &mut base,
        center + Vec3::Y * 6.0,
        center + Vec3::Y * 14.0,
        34.0,
        STONE,
    );
    solid(
        &mut base,
        center + Vec3::Y * 14.0,
        center + Vec3::Y * p.mast_height,
        5.0,
        BRASS,
    );
    let arm_height = p.mast_height * 0.5;
    let arm_end = center + Vec3::new(0.0, arm_height, 38.0);
    solid(
        &mut base,
        center + Vec3::Y * arm_height,
        arm_end,
        3.5,
        BRASS,
    );

    // one blade module shared by both rotors
    let mut blade = VoxelModule::new(4);
    solid(
        &mut blade,
        HUB - Vec3::Y * 5.0,
        HUB + Vec3::Y * 5.0,
        9.0,
        BRASS,
    );
    for i in 0..p.blades {
        let angle = i as f32 / p.blades as f32 * TAU;
        let out = Vec3::new(angle.cos(), 0.0, angle.sin());
        let side = Vec3::Y.cross(out);
        let tip = HUB + out * p.blade_length;
        // a flattened, slightly twisted paddle
        let (min, max) = capsule_bounds(HUB, tip, 14.0);
        fill_sdf(
            &mut blade,
            min,
            max,
            |q| {
                let d = q - HUB;
                let along = d.dot(out);
                let f = (along / p.blade_length).clamp(0.0, 1.0);
                let twist = 0.5 * f;
                let normal = Vec3::Y * twist.cos() + side * twist.sin();
                let thickness = d.dot(normal).abs() - 1.5;
                let width = d.dot(normal.cross(out)).abs() - (4.0 + 9.0 * (f * (1.0 - f)).sqrt());
                thickness.max(width).max(-along).max(along - p.blade_length)
            },
            |q, _| {
                if (q - HUB).dot(out) > p.blade_length * 0.85 {
                    BLADE_TIP
                } else {
                    BLADE
                }
            },
        );
    }

    let top = center + Vec3::Y * (p.mast_height + 6.0);
    CanonicalAssembly {
        name: recipe.name.clone(),
        voxel_size: recipe.voxel_size,
        palette: palette(&[
            ([0.22, 0.21, 0.2], 0.0),
            ([0.55, 0.38, 0.12], 0.0),
            ([0.75, 0.74, 0.7], 0.0),
            ([0.9, 0.2, 0.08], 0.4),
        ]),
        modules: vec![base, blade],
        groups: vec![
            PoseGroupTemplate {
                name: "base".into(),
                parent: None,
                module: 0,
                rest: Affine3A::from_translation(-center),
                motion: Motion::Static,
            },
            // spins flat on top of the mast
            PoseGroupTemplate {
                name: "rotor_top".into(),
                parent: Some(0),
                module: 1,
                rest: Affine3A::from_translation(top - HUB),
                motion: Motion::Spin {
                    pivot: HUB,
                    axis: Vec3::Y,
                    rate: p.rate_a,
                },
            },
            // the same blades, stood upright like a windmill on the side arm
            PoseGroupTemplate {
                name: "rotor_side".into(),
                parent: Some(0),
                module: 1,
                rest: Affine3A::from_translation(arm_end + Vec3::Z * 6.0)
                    * Affine3A::from_quat(Quat::from_rotation_x(FRAC_PI_2))
                    * Affine3A::from_scale(Vec3::splat(0.6))
                    * Affine3A::from_translation(-HUB),
                motion: Motion::Spin {
                    pivot: HUB,
                    axis: Vec3::Y,
                    rate: p.rate_b,
                },
            },
        ],
    }
}
