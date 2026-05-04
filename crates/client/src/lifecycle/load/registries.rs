use super::AppStartupPhase;
use crate::{
    prelude::*,
    render::{
        block::BlockRenderDataRegistry,
        texture::{BlockTextureArray, BlockTextureProcessor},
    },
};
use bevy::{asset::Assets, ecs::world::CommandQueue, prelude::*, tasks::AsyncComputeTaskPool};
use shared::{
    lifecycle::{
        load::{LoadingTaskComponent, NodeFinished, StartNode},
        PersistentPathsResource,
    },
    world::{biome::BiomeRegistryResource, block::BlockRegistry},
};

/// Observer that handles the texture stitching task.
pub fn handle_texture_stitching(
    _trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPathsResource>,
) {
    if _trigger.event().0 != AppStartupPhase::Textures {
        return;
    }

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
            world.trigger(NodeFinished(AppStartupPhase::Textures));
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Textures));
}

/// Observer that handles the block registry loading task.
pub fn handle_block_loading(
    _trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    if _trigger.event().0 != AppStartupPhase::Blocks {
        return;
    }

    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let block_registry = BlockRegistry::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(block_registry);
            world.trigger(NodeFinished(AppStartupPhase::Blocks));
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Blocks));
}

/// Observer that handles the biome registry loading task.
pub fn handle_biome_loading(
    _trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    persistent_paths: Res<PersistentPathsResource>,
) {
    if _trigger.event().0 != AppStartupPhase::Biomes {
        return;
    }

    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let biome_registry = BiomeRegistryResource::load_from_disk(&paths);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(biome_registry);
            world.trigger(NodeFinished(AppStartupPhase::Biomes));
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::Biomes));
}

/// Observer that handles the render registry loading task.
pub fn handle_render_registry(
    _trigger: On<StartNode<AppStartupPhase>>,
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPathsResource>,
) {
    if _trigger.event().0 != AppStartupPhase::RenderRegistry {
        return;
    }

    let settings = client_settings.clone();
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let block_registry = BlockRegistry::load_from_disk(&paths);
        let (_, texture_registry) =
            BlockTextureProcessor::new(paths.assets_dir.clone(), &settings.texture_pack)
                .load_and_stitch()
                .expect("Failed to load textures for render registry!");

        let render_registry =
            BlockRenderDataRegistry::load_from_disk(&paths, &block_registry, &texture_registry);

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            world.insert_resource(render_registry);
            world.trigger(NodeFinished(AppStartupPhase::RenderRegistry));
        });
        queue
    });

    // Spawn a dedicated entity to track this task
    commands.spawn((LoadingTaskComponent(task), AppStartupPhase::RenderRegistry));
}
