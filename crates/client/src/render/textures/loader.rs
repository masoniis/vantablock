use crate::prelude::*;
use crate::render::textures::error::TextureLoadError;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::Image;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use image::RgbaImage;
use shared::ecs_core::config::AppConfig;
use shared::simulation::block::texture_registry::{TextureId, TextureRegistryResource};
use std::collections::HashMap;

// INFO: ----------------------------
//         public loading API
// ----------------------------------

/// Loads texture assets from disk, stitches them into a 2D Texture Array,
/// and returns the native Bevy Image alongside the registry.
pub fn load_voxel_texture_assets(
    config: &AppConfig,
) -> Result<(Image, TextureRegistryResource), TextureLoadError> {
    info!("Loading texture assets from disk...");

    // load loose images
    let (images, texture_map) = load_images_from_disk(&config.texture_pack)?;

    // validate
    let (width, height) = determine_texture_dimensions(&images)?;
    validate_image_dimensions(&images, width, height)?;

    // Combine all pixel data into a single flat byte vector for the array
    let layer_count = images.len() as u32;
    let mut all_bytes = Vec::with_capacity((width * height * 4 * layer_count) as usize);
    for img in &images {
        all_bytes.extend_from_slice(img.as_raw());
    }

    // Define the Bevy Image as a 2D Array
    let mut texture_array = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: layer_count,
        },
        TextureDimension::D2,
        all_bytes,
        TextureFormat::Rgba8UnormSrgb,
        // Drop the CPU memory once it is uploaded to the GPU!
        RenderAssetUsages::RENDER_WORLD,
    );

    // Explicitly set the view dimension to 2DArray so Bevy creates the correct view
    texture_array.texture_view_descriptor = Some(TextureViewDescriptor {
        label: Some("Voxel Texture Array View"),
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });

    let registry = TextureRegistryResource::new(texture_map)?;

    Ok((texture_array, registry))
}

// INFO: ----------------------------------
//         private helper functions
// ----------------------------------------

/// Iterates the `TextureId` manifest and loads the corresponding PNG files.
fn load_images_from_disk(
    texture_pack: &str,
) -> Result<(Vec<RgbaImage>, HashMap<String, TextureId>), TextureLoadError> {
    let base_path = get_resource_path("assets/textures");
    let path = base_path.join(texture_pack);
    let glob_path = path.join("*.png");

    let mut images = Vec::new();
    let mut texture_map = HashMap::new();

    // take in and sort paths
    let mut paths: Vec<_> = glob::glob(glob_path.to_str().unwrap())
        .expect("Failed to read glob pattern")
        .filter_map(|e| e.ok())
        .collect();
    paths.sort();

    // get size for images based on first image
    let (w, h) = if let Some(first_path) = paths.first() {
        image::image_dimensions(first_path)
            .map_err(|e| TextureLoadError::ImageError(first_path.display().to_string(), e))?
    } else {
        (2, 2) // default size if folder is empty
    };

    images.push(generate_missing_texture_image(w, h));
    texture_map.insert("missing".to_string(), 0);

    // load remaining
    for (i, path) in paths.iter().enumerate() {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();

        let image = image::open(path)
            .map_err(|e| TextureLoadError::ImageError(path.display().to_string(), e))?
            .to_rgba8();

        images.push(image);

        texture_map.insert(name, (i + 1) as TextureId);
    }

    Ok((images, texture_map))
}

/// Finds the first valid, non-placeholder image to determine the reference dimensions.
fn determine_texture_dimensions(images: &[RgbaImage]) -> Result<(u32, u32), TextureLoadError> {
    images
        .iter()
        .find(|img| img.width() > 0 && img.height() > 0)
        .map(|img| img.dimensions())
        .ok_or(TextureLoadError::NoTexturesFound)
}

/// Validates that all images in the vector match the reference dimensions.
fn validate_image_dimensions(
    images: &[RgbaImage],
    width: u32,
    height: u32,
) -> Result<(), TextureLoadError> {
    for (i, img) in images.iter().enumerate() {
        if img.dimensions() != (width, height) {
            return Err(TextureLoadError::DimensionMismatch(
                i.to_string(),
                img.width(),
                img.height(),
                width,
                height,
            ));
        }
    }
    Ok(())
}

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
