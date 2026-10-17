use super::{BlockFaceTextures, BlockRenderData};
use crate::prelude::*;
use crate::render::texture::{TextureId, TextureRegistryResource};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use shared::lifecycle::PersistentPathsResource;
use shared::lifecycle::load::{LoadingTaskComponent, NodeCompleted, StartNode};
use shared::world::block::{
    AIR_BLOCK_ID, BlockId, BlockRegistry, SOLID_BLOCK_ID, load_block_from_str,
};
use std::sync::Arc;

/// Marker node for loading client-specific render data for blocks.
#[derive(Component)]
pub struct LoadRenderRegistry;

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

/// An asynchronous loading observer for the render registry.
///
/// This is triggered when the render registry loading node is ready to begin.
pub fn handle_render_registry(
    trigger: On<StartNode>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
    block_registry: Res<BlockRegistry>,
    texture_registry: Res<TextureRegistryResource>,
) {
    let paths = persistent_paths.0.clone();
    let blocks = block_registry.clone();
    let textures = texture_registry.clone();
    let node_entity = trigger.0.entity();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let render_registry = BlockRenderDataRegistry::load_from_disk(&paths, &blocks, &textures);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(render_registry);
            world.trigger(NodeCompleted::of::<LoadRenderRegistry>());
        });
        queue
    });

    // Spawn the task as a child of the node entity
    commands.entity(node_entity).with_children(|parent| {
        parent.spawn(LoadingTaskComponent(task));
    });
}
