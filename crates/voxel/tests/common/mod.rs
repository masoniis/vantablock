#![allow(dead_code)]

use glam::{UVec3, Vec3};
use voxel::VoxelModule;

/// A tiny deterministic rng so tests need no extra dependencies.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    pub fn direction(&mut self) -> Vec3 {
        loop {
            let v = Vec3::new(
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
            );
            if v.length_squared() > 0.01 && v.length_squared() <= 1.0 {
                return v.normalize();
            }
        }
    }
}

/// The hierarchy boundaries most likely to break traversal: node edges at every level.
pub const BOUNDARIES: [u32; 9] = [0, 3, 4, 15, 16, 63, 64, 255, 256];

/// A sparse module with voxels clustered on both sides of every node boundary, plus noise.
pub fn boundary_module(levels: u8, rng: &mut Rng) -> VoxelModule {
    let mut module = VoxelModule::new(levels);
    let extent = module.extent();
    let coords: Vec<u32> = BOUNDARIES
        .iter()
        .copied()
        .chain([extent - 1, extent / 2, extent / 2 - 1])
        .filter(|&c| c < extent)
        .collect();
    for &x in &coords {
        for &y in &coords {
            for &z in &coords {
                if rng.below(3) == 0 {
                    module.set(UVec3::new(x, y, z), 1 + rng.below(4) as u16);
                }
            }
        }
    }
    for _ in 0..400 {
        let p = UVec3::new(rng.below(extent), rng.below(extent), rng.below(extent));
        module.set(p, 1 + rng.below(4) as u16);
    }
    module
}

/// A thin, branching "foliage" module: random walks of single voxels.
pub fn foliage_module(levels: u8, rng: &mut Rng) -> VoxelModule {
    let mut module = VoxelModule::new(levels);
    let extent = module.extent() as i32;
    for _ in 0..24 {
        let mut p = glam::IVec3::new(
            rng.below(extent as u32) as i32,
            rng.below(extent as u32) as i32,
            rng.below(extent as u32) as i32,
        );
        for _ in 0..120 {
            if p.cmpge(glam::IVec3::ZERO).all() && p.cmplt(glam::IVec3::splat(extent)).all() {
                module.set(p.as_uvec3(), 2);
            }
            p[rng.below(3) as usize] += if rng.below(2) == 0 { 1 } else { -1 };
        }
    }
    module
}
