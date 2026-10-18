//! Meadow ground tiles: a soil surface covered in individual microvoxel grass blades, pebbles,
//! and flowers. Tiles are instanced (with rotations) to carpet the ground.

use crate::{
    AssetRecipe,
    generators::palette,
    noise::{Rng, fbm},
};
use glam::{Affine3A, UVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::{CanonicalAssembly, Motion, PoseGroupTemplate, VoxelModule};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MeadowParams {
    /// Tile width in voxels.
    pub size: u32,
    /// Grass blades per square voxel.
    pub grass_density: f32,
    pub grass_height: f32,
    pub flowers: u32,
    pub pebbles: u32,
}

impl Default for MeadowParams {
    fn default() -> Self {
        Self {
            size: 512,
            grass_density: 0.09,
            grass_height: 22.0,
            flowers: 90,
            pebbles: 160,
        }
    }
}

const SOIL: [u16; 2] = [1, 2];
const GRASS: [u16; 4] = [3, 4, 5, 6];
const PEBBLE: u16 = 7;
const PETALS: [u16; 3] = [8, 9, 10];
const SOIL_DEPTH: u32 = 3;
/// Height of the soil surface inside the tile, leaving room below for the gentle relief.
const SURFACE: f32 = 6.0;

pub fn generate(recipe: &AssetRecipe, p: &MeadowParams) -> CanonicalAssembly {
    let mut rng = Rng::new(recipe.seed);
    let seed = recipe.seed as u32;
    let mut module = VoxelModule::with_extent(p.size);
    let size = p.size.min(module.extent());

    // gentle relief that tiles seamlessly would need periodic noise; keep it low so seams vanish
    let height_at = |x: u32, z: u32| -> u32 {
        let edge = (x.min(size - 1 - x).min(z).min(size - 1 - z) as f32 / 24.0).min(1.0);
        (SURFACE + edge * 3.0 * (fbm(Vec3::new(x as f32, 0.0, z as f32) * 0.012, 3, seed) - 0.5))
            as u32
    };

    for z in 0..size {
        for x in 0..size {
            let top = height_at(x, z);
            let mottle = fbm(Vec3::new(x as f32, 1.0, z as f32) * 0.05, 2, seed ^ 3);
            for y in top.saturating_sub(SOIL_DEPTH)..=top {
                module.set(UVec3::new(x, y, z), SOIL[(mottle > 0.55) as usize]);
            }
        }
    }

    // grass: thin, bending, tapering single-voxel blades
    let blades = (size as f32 * size as f32 * p.grass_density) as u32;
    for _ in 0..blades {
        let (x, z) = (rng.range(0.0, size as f32), rng.range(0.0, size as f32));
        let lush = fbm(Vec3::new(x, 2.0, z) * 0.02, 3, seed ^ 5);
        let height = p.grass_height * rng.range(0.35, 1.0) * (0.5 + lush);
        let lean = Vec3::new(rng.range(-1.0, 1.0), 0.0, rng.range(-1.0, 1.0)) * 0.35;
        let material = GRASS[((lush * 2.5 + rng.range(0.0, 1.5)) as usize).min(3)];
        let mut pos = Vec3::new(x, height_at(x as u32, z as u32) as f32 + 1.0, z);
        for i in 0..height as u32 {
            let f = i as f32 / height;
            let p = pos.floor();
            if p.min_element() >= 0.0 && p.x < size as f32 && p.z < size as f32 {
                module.set(p.as_uvec3(), material);
            }
            // blades bend further the higher they go
            pos += Vec3::Y + lean * f * 2.0;
        }
    }

    for _ in 0..p.pebbles {
        let c = Vec3::new(
            rng.range(4.0, size as f32 - 4.0),
            0.0,
            rng.range(4.0, size as f32 - 4.0),
        );
        let r = rng.range(1.5, 4.0);
        let base = height_at(c.x as u32, c.z as u32) as f32;
        sphere(&mut module, c + Vec3::Y * base, r, PEBBLE);
    }

    for _ in 0..p.flowers {
        let (x, z) = (
            rng.range(3.0, size as f32 - 3.0),
            rng.range(3.0, size as f32 - 3.0),
        );
        let stem = rng.range(10.0, 26.0);
        let base = height_at(x as u32, z as u32) as f32 + 1.0;
        for i in 0..stem as u32 {
            module.set(Vec3::new(x, base + i as f32, z).as_uvec3(), GRASS[1]);
        }
        sphere(
            &mut module,
            Vec3::new(x, base + stem, z),
            rng.range(1.5, 2.6),
            PETALS[rng.below(3) as usize],
        );
    }

    CanonicalAssembly {
        name: recipe.name.clone(),
        voxel_size: recipe.voxel_size,
        palette: palette(&[
            ([0.09, 0.06, 0.04], 0.0),
            ([0.12, 0.08, 0.05], 0.0),
            ([0.05, 0.18, 0.03], 0.0),
            ([0.08, 0.26, 0.04], 0.0),
            ([0.16, 0.34, 0.06], 0.0),
            ([0.30, 0.38, 0.10], 0.0),
            ([0.30, 0.29, 0.27], 0.0),
            ([0.85, 0.82, 0.25], 0.0),
            ([0.80, 0.25, 0.30], 0.0),
            ([0.55, 0.45, 0.85], 0.0),
        ]),
        modules: vec![module],
        groups: vec![PoseGroupTemplate {
            name: "tile".into(),
            parent: None,
            module: 0,
            // anchored at the tile's center, on the soil surface
            rest: Affine3A::from_translation(-Vec3::new(
                size as f32 * 0.5,
                SURFACE + 1.0,
                size as f32 * 0.5,
            )),
            motion: Motion::Static,
        }],
    }
}

fn sphere(module: &mut VoxelModule, c: Vec3, r: f32, material: u16) {
    let lo = (c - Vec3::splat(r)).floor().max(Vec3::ZERO).as_uvec3();
    let hi = (c + Vec3::splat(r)).ceil().as_uvec3();
    for z in lo.z..hi.z {
        for y in lo.y..hi.y {
            for x in lo.x..hi.x {
                if (Vec3::new(x as f32, y as f32, z as f32) + 0.5).distance(c) <= r {
                    module.set(UVec3::new(x, y, z), material);
                }
            }
        }
    }
}
