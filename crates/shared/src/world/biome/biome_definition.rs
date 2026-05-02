use bevy::asset::Asset;
use bevy::reflect::TypePath;
use serde::Deserialize;

/// Loads a `BiomeDefinition` struct from a RON string.
///
/// Handles the entire raw ron -> type `BiomeDefinition` conversion process.
pub fn load_biome_from_str(ron_string: &str) -> Result<BiomeDefinition, ron::Error> {
    let raw_definition: raw::BiomeDefinition = ron::from_str(ron_string)?;
    Ok(raw_definition.into())
}

// INFO: -------------------------------------
//         the biome struct definition
// -------------------------------------------

#[derive(Asset, TypePath, Debug, Clone, Deserialize)]
pub struct BiomeDefinition {
    pub name: String,
    pub tint_colors: BiomeTintColors,
    pub terrain: TerrainParameters,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BiomeTintColors {
    pub grass: Option<[f32; 3]>,
    pub water: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TerrainParameters {
    pub surface_material: String,
    pub subsurface_material: String,
}

mod raw {
    use super::*;

    // INFO: -----------------------------------------------
    //         raw deserialization struct definition
    // -----------------------------------------------------

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub(super) struct BiomeDefinition {
        pub(super) name: String,

        #[serde(default)]
        pub(super) tint_colors: Option<RawTintColors>,

        pub(super) terrain: RawTerrainParameters,
    }

    #[derive(Deserialize, Debug, Default)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RawTintColors {
        #[serde(default)]
        pub(super) grass: Option<[f32; 3]>,
        #[serde(default)]
        pub(super) water: Option<[f32; 3]>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RawTerrainParameters {
        pub(super) surface_material: String,
        pub(super) subsurface_material: String,
    }

    // INFO: ----------------------------------------------------------------
    //         conversion from raw struct to concrete BiomeDefinition
    // ----------------------------------------------------------------------

    impl From<raw::BiomeDefinition> for super::BiomeDefinition {
        fn from(raw_def: raw::BiomeDefinition) -> Self {
            Self {
                name: raw_def.name,
                tint_colors: raw_def
                    .tint_colors
                    .map_or_else(BiomeTintColors::default, |raw| raw.into()),
                terrain: raw_def.terrain.into(),
            }
        }
    }

    impl From<raw::RawTintColors> for super::BiomeTintColors {
        fn from(raw_tints: raw::RawTintColors) -> Self {
            Self {
                grass: raw_tints.grass,
                water: raw_tints.water,
            }
        }
    }

    impl From<raw::RawTerrainParameters> for super::TerrainParameters {
        fn from(raw_params: raw::RawTerrainParameters) -> Self {
            Self {
                surface_material: raw_params.surface_material,
                subsurface_material: raw_params.subsurface_material,
            }
        }
    }
}
