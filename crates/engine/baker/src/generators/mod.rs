//! Procedural generators. Each turns a seed and parameters into canonical modules plus a pose
//! group assembly. Generators are behavior-driven: a tree's boughs are ordinary hinged pose
//! groups, exactly like any other articulated part.

pub mod crystal;
pub mod grotto;
pub mod meadow;
pub mod rotor;
pub mod tree;

use voxel::Material;

/// Builds a palette from linear RGB colors, reserving slot 0 for empty space.
pub(crate) fn palette(colors: &[([f32; 3], f32)]) -> Vec<Material> {
    std::iter::once(Material::new([0.0; 3]))
        .chain(colors.iter().map(|&(albedo, emission)| Material {
            albedo,
            roughness: 0.8,
            emission,
        }))
        .collect()
}
