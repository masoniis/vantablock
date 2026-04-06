use crate::{
    prelude::*,
    simulation::biome::biome_definition::{BiomeDefinition, load_biome_from_str},
};
use bevy::ecs::prelude::*;
use std::{collections::HashMap, fs, sync::Arc};

pub type BiomeId = u8;

#[derive(Resource, Default, Clone)]
pub struct BiomeRegistryResource {
    /// Stores definitions indexed by BiomeId enum variant.
    definitions: Arc<Vec<BiomeDefinition>>,

    /// Maps a string name to the runtime ID.
    name_to_id: Arc<HashMap<String, BiomeId>>,
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
}

// INFO: ------------------------------
//         System to load files
// ------------------------------------

/// A system that is built to run once at startup. It scans the biome directory and
/// loads all definitions found into the `BiomeRegistryResource` for global access.
#[instrument(skip_all)]
pub fn initialize_biome_registry_system(mut commands: Commands) {
    let registry = load_biome_defs_from_disk();
    commands.insert_resource(registry);
}

/// A util that scans the biome asset directory and loads all valid biome definitions
/// found into a `BiomeRegistryResource` struct.
#[instrument(skip_all)]
pub fn load_biome_defs_from_disk() -> BiomeRegistryResource {
    info!("Loading biome definitions...");

    let mut biome_definitions: Vec<BiomeDefinition> = Vec::new();
    let mut name_to_id: HashMap<String, BiomeId> = HashMap::new();
    let biome_dir = get_resource_path("assets/biomes");

    // helper closure for local registration (identical to block loading)
    let mut register = |name: String, definition: BiomeDefinition| -> BiomeId {
        let id = biome_definitions.len() as BiomeId;
        biome_definitions.push(definition);
        name_to_id.insert(name.to_lowercase(), id);
        id
    };

    // load the default biome
    let default_biome_name = "ocean";
    let default_ron_path = biome_dir.join(format!("{}.ron", default_biome_name));
    let default_definition = match fs::read_to_string(&default_ron_path)
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
    info!("Registered default biome '{}' as ID 0", default_biome_name);

    // now parse the rest of the biomes
    if biome_dir.is_dir() {
        for entry in fs::read_dir(&biome_dir).unwrap_or_else(|e| {
            panic!("Failed to read biome directory {:?}: {}", biome_dir, e);
        }) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read entry in biome directory: {}", e);
                    continue;
                }
            };
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|s| s == "biome" || s == "ron") {
                // name is the file stem
                let name = match path.file_stem().and_then(|s| s.to_str()) {
                    Some(name_str) => name_str.to_string(),
                    None => {
                        warn!(
                            "Skipping biome definition with invalid filename: {:?}",
                            path.file_name()
                        );
                        continue;
                    }
                };

                // skip default or _ files
                if name == default_biome_name || name.starts_with("_") {
                    continue;
                }

                let ron_string = match fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to read biome file {:?}: {}", path, e);
                        continue;
                    }
                };

                // construct concrete biome definition object
                match load_biome_from_str(&ron_string) {
                    Ok(definition) => {
                        let runtime_id = register(name.clone(), definition);
                        info!("Loaded biome '{}' (runtime id={})", name, runtime_id);
                    }
                    Err(e) => {
                        error!("Failed to parse biome file {:?}: {}", path, e);
                    }
                }
            }
        }
    } else {
        warn!(
            "Biome directory not found at: {:?}. Only default biome was loaded.",
            biome_dir
        );
    }

    let registry = BiomeRegistryResource {
        definitions: Arc::new(biome_definitions),
        name_to_id: Arc::new(name_to_id),
    };

    if registry.definitions.len() <= 1 {
        warn!(
            "Only the default biome was loaded. Check 'assets/biomes/' directory for other biome files."
        );
    }

    registry
}
