use crate::{
    prelude::*,
    simulation::block::texture_registry::{TextureId, TextureRegistryResource},
    simulation::block::{
        BlockDescription, BlockFaceTextures, BlockRenderData, load_block_from_str,
    },
};
use bevy::asset::AssetServer;
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
pub struct BlockRegistryResource {
    /// All loaded block render data from disc.
    render_data: Arc<Vec<BlockRenderData>>,
    /// Direct access to transparency data from BlockRenderData
    /// to optimize super hot loops (meshing).
    transparency_lut: Arc<Vec<bool>>,
    /// Direct access to slices of TextureIds (Hot Array)
    /// to optimize super hot loops (meshing).
    /// Layout: [Top, Bottom, Left, Right, Front, Back]
    texture_lut: Arc<Vec<[TextureId; 6]>>,

    /// All loaded block descriptors from disc.
    descriptions: Arc<Vec<BlockDescription>>,

    /// Maps a string name to the runtime ID.
    name_to_id: Arc<HashMap<String, BlockId>>,
}

impl FromWorld for BlockRegistryResource {
    fn from_world(world: &mut World) -> Self {
        let texture_registry = world.get_resource::<TextureRegistryResource>();
        let persistent_paths = world
            .get_resource::<PersistentPaths>()
            .cloned()
            .unwrap_or_else(PersistentPaths::resolve);
        Self::load_from_disk(texture_registry, &persistent_paths)
    }
}

impl BlockRegistryResource {
    /// Gets the render data for a given block ID.
    ///
    /// Will have undefined behavior if the block ID is not within bounds.
    #[inline(always)]
    pub fn get_render_data(&self, id: BlockId) -> &BlockRenderData {
        unsafe { self.render_data.get_unchecked(id as usize) }
    }

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

    /// Returns a slice of texture arrays for all blocks.
    /// Index is BlockId.
    ///
    /// Use this for meshing to ensure O(1) array indexing without branching.
    #[inline(always)]
    pub fn get_texture_lut(&self) -> &[[TextureId; 6]] {
        &self.texture_lut
    }

    /// Internal util to load all blocks from disk into a new registry instance.
    pub fn load_from_disk(
        texture_registry: Option<&TextureRegistryResource>,
        persistent_paths: &PersistentPaths,
    ) -> Self {
        info!("Loading block definitions from disk...");

        let mut render_data_vec: Vec<BlockRenderData<TextureId>> = Vec::new();
        let mut descriptions_vec: Vec<BlockDescription> = Vec::new();
        let mut texture_lut_vec: Vec<[TextureId; 6]> = Vec::new();
        let mut name_to_id: HashMap<String, BlockId> = HashMap::new();

        // INFO: ---------------------------------------
        //          manual air block registration (ID 0)
        // ---------------------------------------------

        let air_render = BlockRenderData {
            is_transparent: true,
            textures: BlockFaceTextures {
                front: "missing".to_string(),
                back: "missing".to_string(),
                right: "missing".to_string(),
                left: "missing".to_string(),
                top: "missing".to_string(),
                bottom: "missing".to_string(),
            },
        };

        let air_desc = BlockDescription {
            display_name: "Air".to_string(),
        };

        let air_id = register_block(
            "air".to_string(),
            air_render,
            air_desc,
            None,
            texture_registry,
            &mut render_data_vec,
            &mut descriptions_vec,
            &mut texture_lut_vec,
            &mut name_to_id,
        );
        if air_id != AIR_BLOCK_ID {
            panic!("Critical: Air block was not registered as ID 0.");
        }

        // INFO: ----------------------------------------
        //          reserve stone block ID (ID 1)
        // ----------------------------------------------

        let missing_texture_id = texture_registry.map(|r| r.get_id("missing")).unwrap_or(0);

        let placeholder_render_data_ids = BlockRenderData {
            is_transparent: false,
            textures: BlockFaceTextures {
                front: missing_texture_id,
                back: missing_texture_id,
                right: missing_texture_id,
                left: missing_texture_id,
                top: missing_texture_id,
                bottom: missing_texture_id,
            },
        };

        let placeholder_desc = BlockDescription {
            display_name: "Stone (Placeholder)".to_string(),
        };

        render_data_vec.push(placeholder_render_data_ids);
        descriptions_vec.push(placeholder_desc);
        texture_lut_vec.push([missing_texture_id; 6]);

        // INFO: ------------------------------------------
        //         parse block from shared assets
        // ------------------------------------------------

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
                                render_props,
                                desc_props,
                                Some(SOLID_BLOCK_ID),
                                texture_registry,
                                &mut render_data_vec,
                                &mut descriptions_vec,
                                &mut texture_lut_vec,
                                &mut name_to_id,
                            );
                            stone_was_loaded = true;
                        } else {
                            register_block(
                                name.clone(),
                                render_props,
                                desc_props,
                                None,
                                texture_registry,
                                &mut render_data_vec,
                                &mut descriptions_vec,
                                &mut texture_lut_vec,
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

        let transparency_lut: Vec<bool> =
            render_data_vec.iter().map(|d| d.is_transparent).collect();

        Self {
            render_data: Arc::new(render_data_vec),
            transparency_lut: Arc::new(transparency_lut),
            texture_lut: Arc::new(texture_lut_vec),
            descriptions: Arc::new(descriptions_vec),
            name_to_id: Arc::new(name_to_id),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn register_block(
    name: String,
    render: BlockRenderData<String>,
    desc: BlockDescription,
    force_id: Option<BlockId>,
    texture_registry: Option<&TextureRegistryResource>,
    render_data_vec: &mut Vec<BlockRenderData<TextureId>>,
    descriptions_vec: &mut Vec<BlockDescription>,
    texture_lut_vec: &mut Vec<[TextureId; 6]>,
    name_to_id: &mut HashMap<String, BlockId>,
) -> BlockId {
    // resolve strings from parsed ron to textures
    let resolved_textures = if let Some(registry) = texture_registry {
        render.textures.map(|n| registry.get_id(&n))
    } else {
        BlockFaceTextures {
            front: 0,
            back: 0,
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        }
    };

    let render_with_ids = BlockRenderData {
        is_transparent: render.is_transparent,
        textures: resolved_textures.clone(),
    };

    // hot texture array
    let texture_array = [
        resolved_textures.top,
        resolved_textures.bottom,
        resolved_textures.right,
        resolved_textures.left,
        resolved_textures.front,
        resolved_textures.back,
    ];

    // force id into slot
    if let Some(target_id) = force_id {
        let idx = target_id as usize;
        if idx < render_data_vec.len() {
            render_data_vec[idx] = render_with_ids;
            descriptions_vec[idx] = desc;
            texture_lut_vec[idx] = texture_array;
            name_to_id.insert(name.to_lowercase(), target_id);
            target_id
        } else {
            panic!(
                "Critical: Attempted to force block '{}' to ID {} but registry length is {}",
                name,
                target_id,
                render_data_vec.len()
            );
        }
    } else {
        let id = render_data_vec.len() as BlockId;
        render_data_vec.push(render_with_ids);
        descriptions_vec.push(desc);
        texture_lut_vec.push(texture_array);
        name_to_id.insert(name.to_lowercase(), id);
        id
    }
}

/// A system that scans the block directory and loads all definitions found.
/// This system replaces the manual `FromWorld` implementation to support standard AssetServer pathing.
#[instrument(skip_all)]
pub fn initialize_block_registry_system(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    texture_registry: Option<Res<TextureRegistryResource>>,
    persistent_paths: Res<PersistentPaths>,
) {
    let registry =
        BlockRegistryResource::load_from_disk(texture_registry.as_deref(), &persistent_paths);
    commands.insert_resource(registry);
}
