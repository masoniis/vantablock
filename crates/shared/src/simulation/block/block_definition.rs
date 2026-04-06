use crate::simulation::block::texture_registry::TextureId;
use serde::Deserialize;

/// Loads a block definition from string and returns two hot/cold split structs
pub fn load_block_from_str(
    ron_string: &str,
) -> Result<(BlockRenderData<String>, BlockDescription), ron::Error> {
    let raw_properties: raw::BlockProperties = ron::from_str(ron_string)?;
    Ok(raw_properties.split_into_components())
}

/// Optimized hot block data required for meshing and rendering.
#[derive(Debug, Clone)]
pub struct BlockRenderData<T = TextureId> {
    pub textures: BlockFaceTextures<T>,
    pub is_transparent: bool,
}

/// Cold "heavy" block metadata.
#[derive(Debug, Clone)]
pub struct BlockDescription {
    pub display_name: String,
}

// INFO: ------------------
//         subtypes
// ------------------------

/// The textures associated with each face of a particular block type.
#[derive(Debug, Clone)]
pub struct BlockFaceTextures<T> {
    pub top: T,
    pub bottom: T,
    pub front: T,
    pub back: T,
    pub right: T,
    pub left: T,
}

impl<T: Clone> BlockFaceTextures<T> {
    pub fn map<U, F>(self, mut f: F) -> BlockFaceTextures<U>
    where
        F: FnMut(T) -> U,
    {
        BlockFaceTextures {
            top: f(self.top.clone()),
            bottom: f(self.bottom.clone()),
            front: f(self.front.clone()),
            back: f(self.back.clone()),
            right: f(self.right.clone()),
            left: f(self.left.clone()),
        }
    }

    #[inline(always)]
    pub fn get(&self, face_index: usize) -> T {
        match face_index {
            0 => self.top.clone(),
            1 => self.bottom.clone(),
            2 => self.left.clone(),
            3 => self.right.clone(),
            4 => self.front.clone(),
            _ => self.back.clone(),
        }
    }
}

// INFO: -------------------------
//         deserialization
// -------------------------------

pub mod raw {
    use super::*;

    /// A struct that matches the structure of the block RON files
    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct BlockProperties {
        pub(super) display_name: String,
        pub(super) textures: TextureConfig,
        pub(super) is_transparent: bool,
    }

    impl BlockProperties {
        /// Consumes the raw struct and returns hot/cold separated components.
        pub fn split_into_components(
            self,
        ) -> (super::BlockRenderData<String>, super::BlockDescription) {
            let render_data = super::BlockRenderData {
                textures: self.textures.resolve(),
                is_transparent: self.is_transparent,
            };

            let description = super::BlockDescription {
                display_name: self.display_name,
            };

            (render_data, description)
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct TextureConfig {
        pub(super) fallback: String,

        #[serde(default)]
        pub top: Option<String>,
        #[serde(default)]
        pub bottom: Option<String>,
        #[serde(default)]
        pub front: Option<String>,
        #[serde(default)]
        pub back: Option<String>,
        #[serde(default)]
        pub right: Option<String>,
        #[serde(default)]
        pub left: Option<String>,
    }

    impl TextureConfig {
        pub(super) fn resolve(self) -> BlockFaceTextures<String> {
            let fallback = self.fallback;

            BlockFaceTextures {
                top: self.top.unwrap_or_else(|| fallback.clone()),
                bottom: self.bottom.unwrap_or_else(|| fallback.clone()),
                front: self.front.unwrap_or_else(|| fallback.clone()),
                back: self.back.unwrap_or_else(|| fallback.clone()),
                right: self.right.unwrap_or_else(|| fallback.clone()),
                left: self.left.unwrap_or(fallback),
            }
        }
    }
}
