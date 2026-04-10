use crate::prelude::*;
use crate::render::{
    block::BlockRenderDataRegistry,
    texture::{BlockTextureArray, VoxelTextureProcessor},
};
use bevy::ecs::world::CommandQueue;
use bevy::{
    asset::Assets,
    prelude::{Commands, Image, Res, World},
    tasks::AsyncComputeTaskPool,
};
use shared::{lifecycle::load::LoadingTaskComponent, simulation::block::BlockRegistry};
use utils::PersistentPaths;

/// A system that starts the asynchronous initialization of texture and block registries
pub fn start_async_registry_initialization(
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPaths>,
) {
    info!("Starting asynchronous simulation registry initialization...");

    let settings = client_settings.clone();
    let paths = persistent_paths.clone();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        info!("Initializing simulation registries in background...");

        // texture stitching
        let (texture_array_image, texture_registry) =
            VoxelTextureProcessor::new(paths.assets_dir.clone(), &settings.texture_pack)
                .load_and_stitch()
                .expect("Failed to load and stitch textures");

        // independent simulation block loading
        let block_registry = BlockRegistry::load_from_disk(&paths);

        // independent client render block loading (resolves IDs via simulation registry)
        let render_registry =
            BlockRenderDataRegistry::load_from_disk(&paths, &block_registry, &texture_registry);

        info!("Applying simulation registry results to the world.");

        let mut queue = CommandQueue::default();
        queue.push(move |world: &mut World| {
            let mut image_assets = world.resource_mut::<Assets<Image>>();
            let texture_handle = image_assets.add(texture_array_image);

            // insert all registries
            world.insert_resource(texture_registry);
            world.insert_resource(block_registry);
            world.insert_resource(render_registry);

            // insert texture array handle
            world.insert_resource(BlockTextureArray {
                handle: texture_handle,
            });

            info!("Simulation registries initialized successfully.");
        });
        queue
    });

    // register this as a loading task so the game waits for it
    commands.spawn(LoadingTaskComponent(task));
}
