use crate::prelude::*;
use crate::render::texture::error::TextureLoadError;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::Image;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use image::RgbaImage;
use shared::simulation::block::texture_registry::{TextureId, TextureRegistryResource};
use std::collections::HashMap;
use std::path::PathBuf;

/// A utility for loading voxel texture folders and generating resources from them
pub struct VoxelTextureProcessor {
    base_path: PathBuf,
    texture_pack: String,
}

impl VoxelTextureProcessor {
    pub fn new(assets_dir: PathBuf, texture_pack: &str) -> Self {
        Self {
            base_path: assets_dir.join("client/texture"),
            texture_pack: texture_pack.to_string(),
        }
    }

    /// Scans the directory to build a registry of names and IDs for every available texture.
    pub fn create_registry(&self) -> Result<TextureRegistryResource, TextureLoadError> {
        let paths = self.get_sorted_texture_paths()?;
        let mut texture_map = HashMap::new();

        texture_map.insert("missing".to_string(), 0);
        for (i, path) in paths.iter().enumerate() {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            texture_map.insert(name, (i + 1) as TextureId);
        }

        Ok(TextureRegistryResource::new(texture_map)?)
    }

    /// Scans, loads, and stitches all textures into an Image.
    pub fn load_and_stitch(&self) -> Result<(Image, TextureRegistryResource), TextureLoadError> {
        info!("Stitching texture array for pack: {}", self.texture_pack);

        let paths = self.get_sorted_texture_paths()?;
        let registry = self.create_registry()?;

        // Use the paths to load pixels
        let mut images = Vec::with_capacity(paths.len() + 1);

        // load/generate first image to get dimensions of all voxel textures
        let first_pixels = if let Some(first_path) = paths.first() {
            image::open(first_path)
                .map_err(|e| TextureLoadError::ImageError(first_path.display().to_string(), e))?
                .to_rgba8()
        } else {
            RgbaImage::new(2, 2)
        };

        let (width, height) = first_pixels.dimensions();
        images.push(generate_missing_texture_image(width, height));
        images.push(first_pixels);

        // Load the rest
        for path in paths.iter().skip(1) {
            let img = image::open(path)
                .map_err(|e| TextureLoadError::ImageError(path.display().to_string(), e))?
                .to_rgba8();

            if img.dimensions() != (width, height) {
                return Err(TextureLoadError::DimensionMismatch(
                    path.display().to_string(),
                    img.width(),
                    img.height(),
                    width,
                    height,
                ));
            }
            images.push(img);
        }

        // 3. Stitch
        let layer_count = images.len() as u32;
        let mut all_bytes = Vec::with_capacity((width * height * 4 * layer_count) as usize);
        for img in images {
            all_bytes.extend_from_slice(img.as_raw());
        }

        let mut texture_array = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: layer_count,
            },
            TextureDimension::D2,
            all_bytes,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );

        texture_array.texture_view_descriptor = Some(TextureViewDescriptor {
            label: Some("Voxel Texture Array View"),
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

        Ok((texture_array, registry))
    }

    fn get_sorted_texture_paths(&self) -> Result<Vec<PathBuf>, TextureLoadError> {
        let path = self.base_path.join(&self.texture_pack);
        let glob_path = path.join("*.png");

        let mut paths: Vec<_> = glob::glob(glob_path.to_str().unwrap())
            .expect("Failed to read glob pattern")
            .filter_map(|e| e.ok())
            .collect();

        paths.sort();
        Ok(paths)
    }
}

// INFO: ----------------------------------
//         private helper functions
// ----------------------------------------

/// Generates the missing texture programmatically as a purple and black checkerboard pattern.
fn generate_missing_texture_image(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let checker_size = (width / 2).max(1); // 2x2 checkerboard pattern

    for y in 0..height {
        for x in 0..width {
            let checker_x = x / checker_size;
            let checker_y = y / checker_size;
            let is_even = (checker_x + checker_y) % 2 == 0;

            // defaults to transparent so that it works for both opaque and transparency
            let color = if is_even {
                [255, 0, 255, 200] // magenta/purple
            } else {
                [0, 0, 0, 200] // black
            };

            img.put_pixel(x, y, image::Rgba(color));
        }
    }

    img
}
