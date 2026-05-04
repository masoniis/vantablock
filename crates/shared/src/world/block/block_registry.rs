use crate::{
    prelude::*,
    world::block::{BlockDescription, load_block_from_str},
};
use bevy::ecs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use utils::PersistentPaths;

pub type BlockId = u8;
/// ID of the default "air" block.
pub const AIR_BLOCK_ID: BlockId = 0;
/// ID of a default solid block guaranteed to exist (probably stone).
pub const SOLID_BLOCK_ID: BlockId = 1;

#[derive(Resource, Clone)]
pub struct BlockRegistry {
    /// Direct access to transparency data from BlockRenderData
    /// to optimize super hot loops (meshing).
    transparency_lut: Arc<Vec<bool>>,

    /// All loaded block descriptors from disc.
    descriptions: Arc<Vec<BlockDescription>>,

    /// Maps a string name to the runtime ID.
    name_to_id: Arc<HashMap<String, BlockId>>,
}

impl BlockRegistry {
    /// Gets the description/metadata for a given block ID.
    ///
    /// Will have undefined behavior if the block ID is not within bounds.
    #[inline(always)]
    pub fn get_description(&self, id: BlockId) -> &BlockDescription {
        unsafe { self.descriptions.get_unchecked(id as usize) }
    }

    /// Gets the numeric ID for a given block name.
    ///
    /// The string name of a block is based on the block ron-file name
    /// without the extension. Eg: "grass.ron" -> "grass".
    #[inline(always)]
    pub fn get_block_id_by_name(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(&name.to_lowercase()).copied()
    }

    /// Returns a slice of booleans representing the transparency state of all blocks.
    /// Index is BlockId.
    ///
    /// Use this for AO calculation to maximize cache hit rate.
    #[inline(always)]
    pub fn get_transparency_lut(&self) -> &[bool] {
        &self.transparency_lut
    }

    /// Internal util to load all blocks from disk into a new registry instance.
    pub fn load_from_disk(persistent_paths: &PersistentPaths) -> Self {
        info!("Loading block definitions from disk (Simulation)...");

        let mut transparency_vec: Vec<bool> = Vec::new();
        let mut descriptions_vec: Vec<BlockDescription> = Vec::new();
        let mut name_to_id: HashMap<String, BlockId> = HashMap::new();

        // INFO: ---------------------------------------
        //          manual air block registration (ID 0)
        // ---------------------------------------------

        let air_desc = BlockDescription {
            display_name: "Air".to_string(),
        };

        let air_id = register_block(
            "air".to_string(),
            true,
            air_desc,
            None,
            &mut transparency_vec,
            &mut descriptions_vec,
            &mut name_to_id,
        );
        if air_id != AIR_BLOCK_ID {
            panic!("Critical: Air block was not registered as ID 0.");
        }

        // INFO: ----------------------------------------
        //          reserve stone block ID (ID 1)
        // ----------------------------------------------

        let placeholder_desc = BlockDescription {
            display_name: "Stone (Placeholder)".to_string(),
        };

        transparency_vec.push(false);
        descriptions_vec.push(placeholder_desc);
        name_to_id.insert("stone".to_string(), SOLID_BLOCK_ID);

        // INFO: ----------------------------------------
        //         parse block from shared assets
        // ----------------------------------------------

        let mut stone_was_loaded = false;
        let full_path = persistent_paths.assets_dir.join("shared/block");

        if full_path.is_dir() {
            let entries = std::fs::read_dir(&full_path).unwrap_or_else(|e| {
                panic!("Failed to read block directory {:?}: {}", full_path, e);
            });

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|s| s == "ron") {
                    let name = match path.file_stem().and_then(|s| s.to_str()) {
                        Some(name_str) => name_str.to_string(),
                        None => continue,
                    };

                    if name == "air" || name.starts_with('_') {
                        continue;
                    }

                    if let Ok(ron_string) = std::fs::read_to_string(&path)
                        && let Ok((render_props, desc_props)) = load_block_from_str(&ron_string)
                    {
                        if name == "stone" {
                            register_block(
                                name.clone(),
                                render_props.is_transparent,
                                desc_props,
                                Some(SOLID_BLOCK_ID),
                                &mut transparency_vec,
                                &mut descriptions_vec,
                                &mut name_to_id,
                            );
                            stone_was_loaded = true;
                        } else {
                            register_block(
                                name.clone(),
                                render_props.is_transparent,
                                desc_props,
                                None,
                                &mut transparency_vec,
                                &mut descriptions_vec,
                                &mut name_to_id,
                            );
                        }
                    }
                }
            }
        }

        if !stone_was_loaded {
            warn!("'stone.ron' was not found in assets! ID 1 remains a placeholder.");
        }

        Self {
            transparency_lut: Arc::new(transparency_vec),
            descriptions: Arc::new(descriptions_vec),
            name_to_id: Arc::new(name_to_id),
        }
    }

    /// Construct a registry from raw components (used by unified loading tasks)
    pub fn from_raw(
        transparency_lut: Vec<bool>,
        descriptions: Vec<BlockDescription>,
        name_to_id: HashMap<String, BlockId>,
    ) -> Self {
        Self {
            transparency_lut: Arc::new(transparency_lut),
            descriptions: Arc::new(descriptions),
            name_to_id: Arc::new(name_to_id),
        }
    }
}

fn register_block(
    name: String,
    is_transparent: bool,
    desc: BlockDescription,
    force_id: Option<BlockId>,
    transparency_vec: &mut Vec<bool>,
    descriptions_vec: &mut Vec<BlockDescription>,
    name_to_id: &mut HashMap<String, BlockId>,
) -> BlockId {
    // force id into slot
    if let Some(target_id) = force_id {
        let idx = target_id as usize;
        if idx < transparency_vec.len() {
            transparency_vec[idx] = is_transparent;
            descriptions_vec[idx] = desc;
            name_to_id.insert(name.to_lowercase(), target_id);
            target_id
        } else {
            panic!(
                "Critical: Attempted to force block '{}' to ID {} but registry length is {}",
                name,
                target_id,
                transparency_vec.len()
            );
        }
    } else {
        let id = transparency_vec.len() as BlockId;
        transparency_vec.push(is_transparent);
        descriptions_vec.push(desc);
        name_to_id.insert(name.to_lowercase(), id);
        id
    }
}
