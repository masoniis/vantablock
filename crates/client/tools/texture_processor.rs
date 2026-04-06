use image::{ImageBuffer, Rgba};

fn main() {
    // Tint grass textures
    tint_image("assets/textures/faithful/grass_top.png");
    tint_image("assets/textures/rhinestone/grass_top.png");

    // Generate transparent water texture
    create_transparent_water("assets/textures/faithful/water.png");
    create_transparent_water("assets/textures/rhinestone/water.png");
}

fn tint_image(path: &str) {
    let img = match image::open(path) {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            eprintln!("Error opening image {}: {}", path, e);
            return;
        }
    };

    let (width, height) = img.dimensions();
    let mut tinted_img = ImageBuffer::<Rgba<u8>, _>::new(width, height);

    let tint_r = 145u16;
    let tint_g = 189u16;
    let tint_b = 89u16;

    for (x, y, pixel) in img.enumerate_pixels() {
        let original_color = pixel;

        let new_r = ((original_color[0] as u16 * tint_r) / 255) as u8;
        let new_g = ((original_color[1] as u16 * tint_g) / 255) as u8;
        let new_b = ((original_color[2] as u16 * tint_b) / 255) as u8;

        let new_a = original_color[3];

        tinted_img.put_pixel(x, y, Rgba([new_r, new_g, new_b, new_a]));
    }

    if let Err(e) = tinted_img.save(path) {
        eprintln!("Error saving image {}: {}", path, e);
    } else {
        println!("Successfully tinted {}", path);
    }
}

fn create_transparent_water(path: &str) {
    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(32, 32);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([63u8, 118u8, 228u8, 190u8]);
    }

    if let Err(e) = img.save(path) {
        eprintln!("Error saving image {}: {}", path, e);
    } else {
        println!("Successfully created transparent water texture at {}", path);
    }
}
