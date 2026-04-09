use super::{BlockFaceTextures, BlockRenderData};
use crate::prelude::*;
use bevy::prelude::*;
use shared::simulation::block::texture_registry::{TextureId, TextureRegistryResource};
use shared::simulation::block::{
    AIR_BLOCK_ID, BlockId, BlockRegistry, SOLID_BLOCK_ID, load_block_from_str,
};
use std::sync::Arc;
use utils::PersistentPaths;

/// Client-only visual mapping data for blocks.
#[derive(Resource, Clone, Default)]
pub struct BlockRenderDataRegistry {
    /// Hot texture lookup table. Each index is the 6 face textures for a block.
    ///
    /// Layout: [Top, Bottom, Right, Left, Front, Back]
    texture_lut: Arc<Vec<[TextureId; 6]>>,
    /// Full render properties for each block.
    render_data: Arc<Vec<BlockRenderData>>,
}

impl BlockRenderDataRegistry {
    /// Creates a new registry from raw components.
    pub fn from_raw(texture_lut: Vec<[TextureId; 6]>, render_data: Vec<BlockRenderData>) -> Self {
        Self {
            texture_lut: Arc::new(texture_lut),
            render_data: Arc::new(render_data),
        }
    }

    /// Gets the render data for a given block ID.
    #[inline(always)]
    pub fn get_render_data(&self, id: BlockId) -> &BlockRenderData {
        unsafe { self.render_data.get_unchecked(id as usize) }
    }

    /// Returns a slice of texture arrays for all blocks registered.
    ///
    /// The array is index by BlockId.
    #[inline(always)]
    pub fn get_texture_lut(&self) -> &[[TextureId; 6]] {
        &self.texture_lut
    }

    /// Loads block render data from disk, resolving IDs via the regular block (simulation) registry.
    pub fn load_from_disk(
        paths: &PersistentPaths,
        block_registry: &BlockRegistry,
        texture_registry: &TextureRegistryResource,
    ) -> Self {
        info!("Loading block definitions from disk (Rendering)...");

        // initialize vectors with same size as block registry
        let registry_len = block_registry.get_transparency_lut().len();
        let mut texture_lut = vec![[texture_registry.missing_texture(); 6]; registry_len];
        let mut render_data = vec![
            BlockRenderData {
                is_transparent: false,
                textures: BlockFaceTextures {
                    top: texture_registry.missing_texture(),
                    bottom: texture_registry.missing_texture(),
                    front: texture_registry.missing_texture(),
                    back: texture_registry.missing_texture(),
                    left: texture_registry.missing_texture(),
                    right: texture_registry.missing_texture(),
                },
            };
            registry_len
        ];

        // INFO: ----------------------------------------
        //         manual fallbacks (air & stone)
        // ----------------------------------------------

        // air (ID 0)
        texture_lut[AIR_BLOCK_ID as usize] = [texture_registry.missing_texture(); 6];
        render_data[AIR_BLOCK_ID as usize] = BlockRenderData {
            is_transparent: true,
            textures: BlockFaceTextures {
                top: texture_registry.missing_texture(),
                bottom: texture_registry.missing_texture(),
                front: texture_registry.missing_texture(),
                back: texture_registry.missing_texture(),
                left: texture_registry.missing_texture(),
                right: texture_registry.missing_texture(),
            },
        };

        // stone (ID 1) placeholder if not found on disk
        let missing = texture_registry.missing_texture();
        texture_lut[SOLID_BLOCK_ID as usize] = [missing; 6];
        render_data[SOLID_BLOCK_ID as usize] = BlockRenderData {
            is_transparent: false,
            textures: BlockFaceTextures {
                top: missing,
                bottom: missing,
                front: missing,
                back: missing,
                left: missing,
                right: missing,
            },
        };

        // INFO: ----------------------------------------
        //         parse block from shared assets
        // ----------------------------------------------

        let full_path = paths.assets_dir.join("shared/block");
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
                        && let Ok((render_props, _)) = load_block_from_str(&ron_string)
                    {
                        // fetch ID from simulation registry
                        if let Some(id) = block_registry.get_block_id_by_name(&name) {
                            let idx = id as usize;

                            // resolve textures
                            let textures = BlockFaceTextures {
                                top: texture_registry.get_id(&render_props.textures.top),
                                bottom: texture_registry.get_id(&render_props.textures.bottom),
                                front: texture_registry.get_id(&render_props.textures.front),
                                back: texture_registry.get_id(&render_props.textures.back),
                                left: texture_registry.get_id(&render_props.textures.left),
                                right: texture_registry.get_id(&render_props.textures.right),
                            };

                            texture_lut[idx] = [
                                textures.top,
                                textures.bottom,
                                textures.right,
                                textures.left,
                                textures.front,
                                textures.back,
                            ];

                            render_data[idx] = BlockRenderData {
                                is_transparent: render_props.is_transparent,
                                textures,
                            };
                        }
                    }
                }
            }
        }

        Self {
            texture_lut: Arc::new(texture_lut),
            render_data: Arc::new(render_data),
        }
    }
}
