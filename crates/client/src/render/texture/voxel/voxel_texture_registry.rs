use bevy::asset::Handle;
use bevy::ecs::resource::Resource;
use bevy::prelude::Image;
use bevy::render::extract_resource::ExtractResource;
use std::{collections::HashMap, sync::Arc};

/// A numeric ID for a texture, representing its index in the texture array.
pub type TextureId = u32;

pub type TextureLoadError = String;

/// A resource holding the handle to the stitched voxel texture array.
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct BlockTextureArray {
    pub handle: Handle<Image>,
}

/// A registry for looking up texture indices from a texture name.
#[derive(Resource, Clone)]
pub struct TextureRegistryResource {
    /// Maps the texture name to its TextureId (index) in the GPU texture array.
    name_to_id: Arc<HashMap<String, TextureId>>,

    /// The index of the fallback "missing texture" pattern.
    missing_texture_id: TextureId,
}

impl TextureRegistryResource {
    /// Creates a new texture registry from a pre-populated map.
    pub fn new(map: HashMap<String, TextureId>) -> Result<Self, TextureLoadError> {
        let missing_texture_id = *map.get("missing").expect("Missing texture not in map");

        Ok(Self {
            name_to_id: Arc::new(map),
            missing_texture_id,
        })
    }

    /// Gets the texture ID for a given name, returning the missing texture ID if not found.
    pub fn get_id(&self, name: &str) -> TextureId {
        self.name_to_id
            .get(name)
            .copied()
            .unwrap_or(self.missing_texture_id)
    }

    /// Returns the missing texture ID.
    pub fn missing_texture(&self) -> TextureId {
        self.missing_texture_id
    }

    /// Returns true if the registry contains a texture with the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Returns the total number of textures in the registry.
    pub fn len(&self) -> usize {
        self.name_to_id.len()
    }

    /// Returns true if the registry contains no textures.
    pub fn is_empty(&self) -> bool {
        self.name_to_id.is_empty()
    }
}
