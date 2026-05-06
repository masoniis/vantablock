use crate::prelude::*;
use crate::render::texture::block::block_texture_processor::BlockTextureProcessor;
use bevy::asset::{Assets, Handle};
use bevy::ecs::prelude::*;
use bevy::ecs::resource::Resource;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::Image;
use bevy::render::extract_resource::ExtractResource;
use bevy::tasks::AsyncComputeTaskPool;
use shared::lifecycle::PersistentPathsResource;
use shared::lifecycle::load::{LoadingTaskComponent, NodeCompleted, StartNode};
use std::{collections::HashMap, sync::Arc};

/// Marker node for loading and stitching block textures.
#[derive(Component)]
pub struct LoadTextures;

/// A numeric ID for a texture, representing its index in the texture array.
pub type TextureId = u32;

pub type TextureLoadError = String;

/// A resource holding the handle to the stitched block texture array.
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

    pub fn is_empty(&self) -> bool {
        self.name_to_id.is_empty()
    }
}

/// An asynchronous loading observer for the texture registry.
///
/// This is triggered when the texture stitching node is ready to begin.
pub fn handle_texture_stitching(
    trigger: On<StartNode>,
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let settings = client_settings.clone();
    let paths = persistent_paths.0.clone();
    let node_entity = trigger.0.entity();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let (texture_array_image, texture_registry) =
            BlockTextureProcessor::new(paths.assets_dir.clone(), &settings.texture_pack)
                .load_and_stitch()
                .expect("Failed to load and stitch block textures!");

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            let mut image_assets = world.resource_mut::<Assets<Image>>();
            let texture_handle = image_assets.add(texture_array_image);

            world.insert_resource(texture_registry);
            world.insert_resource(BlockTextureArray {
                handle: texture_handle,
            });

            // Signal that this node is finished
            world.trigger(NodeCompleted::of::<LoadTextures>());
        });
        queue
    });

    // Spawn the task as a child of the node entity
    commands.entity(node_entity).with_children(|parent| {
        parent.spawn(LoadingTaskComponent(task));
    });
}
