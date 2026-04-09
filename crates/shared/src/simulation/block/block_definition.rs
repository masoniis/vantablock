use serde::Deserialize;

/// Loads a block definition from string and returns two hot/cold split structs
pub fn load_block_from_str(
    ron_string: &str,
) -> Result<(raw::ParsedRenderProps, BlockDescription), ron::Error> {
    let raw_properties: raw::BlockProperties = ron::from_str(ron_string)?;
    Ok(raw_properties.split_into_components())
}

/// Cold "heavy" block metadata.
#[derive(Debug, Clone)]
pub struct BlockDescription {
    pub display_name: String,
}

// INFO: -------------------------
//         deserialization
// -------------------------------

pub mod raw {
    use super::*;

    /// Properties parsed from the RON file that are relevant for rendering.
    pub struct ParsedRenderProps {
        pub textures: ParsedTextureConfig,
        pub is_transparent: bool,
    }

    pub struct ParsedTextureConfig {
        pub top: String,
        pub bottom: String,
        pub front: String,
        pub back: String,
        pub right: String,
        pub left: String,
    }

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
        pub fn split_into_components(self) -> (ParsedRenderProps, super::BlockDescription) {
            let render_data = ParsedRenderProps {
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
        pub(super) fn resolve(self) -> ParsedTextureConfig {
            let fallback = self.fallback;

            ParsedTextureConfig {
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
