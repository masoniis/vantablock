use crate::{
    lifecycle::PersistentPathsResource,
    prelude::*,
    world::biome::biome_definition::{BiomeDefinition, load_biome_from_str},
};
use bevy::asset::AssetServer;
use bevy::ecs::prelude::*;
use std::{collections::HashMap, sync::Arc};
use utils::PersistentPaths;

pub type BiomeId = u8;

#[derive(Resource, Clone)]
pub struct BiomeRegistryResource {
    /// Stores definitions indexed by BiomeId enum variant.
    definitions: Arc<Vec<BiomeDefinition>>,

    /// Maps a string name to the runtime ID.
    name_to_id: Arc<HashMap<String, BiomeId>>,
}

impl FromWorld for BiomeRegistryResource {
    fn from_world(_world: &mut World) -> Self {
        // Return an empty registry. The actual loading will be done asynchronously
        // via the LoadingAppExt framework.
        Self {
            definitions: Arc::new(Vec::new()),
            name_to_id: Arc::new(HashMap::new()),
        }
    }
}

impl BiomeRegistryResource {
    /// Gets the definition for a given biome ID (u8).
    ///
    /// Defaults to ID 0 if the ID is invalid.
    pub fn get(&self, id: BiomeId) -> &BiomeDefinition {
        self.definitions.get(id as usize).unwrap_or_else(|| {
            // fallback assumes ID 0 is valid
            warn!(
                "Attempted to get invalid BiomeId: {}. Defaulting to ID 0.",
                id
            );
            &self.definitions[0]
        })
    }

    /// Gets the numeric ID for a given biome name.
    ///
    /// The string name of a biome is based on its file name
    /// without the extension. Eg: "ocean.ron" -> "ocean".
    pub fn get_id_by_name(&self, name: &str) -> Option<BiomeId> {
        self.name_to_id.get(&name.to_lowercase()).copied()
    }

    /// Gets the definition for a given biome name (filename).
    ///
    /// Defaults to ID 0's definition if not found.
    pub fn get_by_name(&self, name: &str) -> &BiomeDefinition {
        let id = self.get_id_by_name(name).unwrap_or(0); // Default to ID 0
        self.get(id)
    }

    /// Gets the biome ID for a given biome name (filename).
    ///
    /// Defaults to ID 0 if not found.
    pub fn get_biome_id_or_default(&self, name: &str) -> BiomeId {
        self.get_id_by_name(name).unwrap_or(0)
    }

    /// Internal util to load all biome from disk into a new registry instance.
    pub fn load_from_disk(persistent_paths: &PersistentPaths) -> Self {
        info!("Loading biome definitions from disk...");

        let mut biome_definitions: Vec<BiomeDefinition> = Vec::new();
        let mut name_to_id: HashMap<String, BiomeId> = HashMap::new();

        // helper closure for local registration
        let mut register = |name: String, definition: BiomeDefinition| -> BiomeId {
            let id = biome_definitions.len() as BiomeId;
            biome_definitions.push(definition);
            name_to_id.insert(name.to_lowercase(), id);
            id
        };

        let full_path = persistent_paths.assets_dir.join("shared/biome");

        // load the default biome
        let default_biome_name = "ocean";
        let default_ron_path = full_path.join(format!("{}.ron", default_biome_name));
        let default_definition = match std::fs::read_to_string(&default_ron_path)
            .map_err(|e| e.to_string())
            .and_then(|ron_string| load_biome_from_str(&ron_string).map_err(|e| e.to_string()))
        {
            Ok(def) => def,
            Err(e) => {
                panic!(
                    "CRITICAL: Failed to load or parse default biome file ({:?}): {}. Cannot proceed.",
                    default_ron_path, e
                );
            }
        };
        let default_id = register(default_biome_name.to_string(), default_definition);
        if default_id != 0 {
            panic!(
                "Default biome '{}' did not get ID 0! Check loading logic.",
                default_biome_name
            );
        }

        // now parse the rest of the biomes
        if full_path.is_dir() {
            for entry in std::fs::read_dir(&full_path)
                .unwrap_or_else(|e| {
                    panic!("Failed to read biome directory {:?}: {}", full_path, e);
                })
                .flatten()
            {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|s| s == "ron") {
                    let name = match path.file_stem().and_then(|s| s.to_str()) {
                        Some(name_str) => name_str.to_string(),
                        None => continue,
                    };

                    if name == default_biome_name || name.starts_with("_") {
                        continue;
                    }

                    if let Ok(ron_string) = std::fs::read_to_string(&path)
                        && let Ok(definition) = load_biome_from_str(&ron_string)
                    {
                        register(name.clone(), definition);
                    }
                }
            }
        }

        Self {
            definitions: Arc::new(biome_definitions),
            name_to_id: Arc::new(name_to_id),
        }
    }
}

// INFO: ------------------------------
//         System to load files
// ------------------------------------

/// A system that is built to run once at startup. It scans the biome directory and
/// loads all definitions found into the `BiomeRegistryResource` for global access.
#[instrument(skip_all)]
pub fn initialize_biome_registry_system(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let registry = BiomeRegistryResource::load_from_disk(&persistent_paths);
    commands.insert_resource(registry);
}
