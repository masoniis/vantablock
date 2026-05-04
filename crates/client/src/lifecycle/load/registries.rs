use super::AppStartupPhase;
use crate::{
    prelude::*,
    render::{
        block::BlockRenderDataRegistry,
        texture::{BlockTextureArray, BlockTextureProcessor, TextureRegistryResource},
    },
};
use bevy::{asset::Assets, ecs::world::CommandQueue, prelude::*, tasks::AsyncComputeTaskPool};
use shared::{
    lifecycle::{
        PersistentPathsResource,
        load::{LoadingTaskComponent, NodeFinished, StartNode},
    },
    world::{biome::BiomeRegistryResource, block::BlockRegistry},
};

/// Observer that handles the texture stitching task.
pub fn handle_texture_stitching(
    trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let entity = trigger.event().entity;
    let settings = client_settings.clone();
    let paths = persistent_paths.clone();

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
            world.trigger(NodeFinished {
                node: AppStartupPhase::Textures,
                entity,
            });
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Textures));
}

/// Observer that handles the block registry loading task.
pub fn handle_block_loading(
    trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let entity = trigger.event().entity;
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let block_registry = BlockRegistry::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(block_registry);
            world.trigger(NodeFinished {
                node: AppStartupPhase::Blocks,
                entity,
            });
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Blocks));
}

/// Observer that handles the biome registry loading task.
pub fn handle_biome_loading(
    trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    let entity = trigger.event().entity;
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let biome_registry = BiomeRegistryResource::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(biome_registry);
            world.trigger(NodeFinished {
                node: AppStartupPhase::Biomes,
                entity,
            });
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Biomes));
}

/// Observer that handles the render registry loading task.
pub fn handle_render_registry(
    trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
    block_registry: Res<BlockRegistry>,
    texture_registry: Res<TextureRegistryResource>,
) {
    let entity = trigger.event().entity;
    let paths = persistent_paths.clone();
    let blocks = block_registry.clone();
    let textures = texture_registry.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let render_registry = BlockRenderDataRegistry::load_from_disk(&paths, &blocks, &textures);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(render_registry);
            world.trigger(NodeFinished {
                node: AppStartupPhase::RenderRegistry,
                entity,
            });
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::RenderRegistry));
}
