use std::fmt;
use std::io::Error;

#[derive(Debug)]
pub enum TextureLoadError {
    IoError(Error),
    ImageError(String, image::ImageError),
    NoTexturesFound,
    DimensionMismatch(String, u32, u32, u32, u32),
    RegistryError(String),
}

impl From<Error> for TextureLoadError {
    fn from(err: Error) -> Self {
        TextureLoadError::IoError(err)
    }
}

impl From<String> for TextureLoadError {
    fn from(err: String) -> Self {
        TextureLoadError::RegistryError(err)
    }
}

impl fmt::Display for TextureLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureLoadError::IoError(err) => {
                write!(f, "Texture loading IO error: {}", err)
            }
            TextureLoadError::ImageError(path, err) => {
                write!(f, "Failed to open or decode image at '{}': {}", path, err)
            }
            TextureLoadError::NoTexturesFound => {
                write!(f, "No valid textures found to determine dimensions.")
            }
            TextureLoadError::DimensionMismatch(name, w, h, exp_w, exp_h) => {
                write!(
                    f,
                    "Texture '{}' has dimensions {}x{}, but expected {}x{}.",
                    name, w, h, exp_w, exp_h
                )
            }
            TextureLoadError::RegistryError(err) => {
                write!(f, "Texture registry error: {}", err)
            }
        }
    }
}

impl std::error::Error for TextureLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TextureLoadError::IoError(err) => Some(err),
            TextureLoadError::ImageError(_, err) => Some(err),
            _ => None,
        }
    }
}
