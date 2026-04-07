use bevy::{
    app::{App, FixedUpdate, PostUpdate, PreStartup},
    asset::Assets,
    log::LogPlugin,
    prelude::{
        default, info, AssetPlugin, DefaultPlugins, Image, IntoScheduleConfigs, PluginGroup,
        Window, WindowPlugin, World,
    },
    window::WindowResolution,
};
use client::{
    input::InputModulePlugin,
    player::PlayerPlugin,
    prelude::*,
    render::{
        texture::{BlockTextureArray, VoxelTextureProcessor},
        VantablockRenderPlugin,
    },
    showcase::ShowcasePlugin,
};
use shared::{
    ecs_core::LoadingTracker,
    simulation::{
        app_lifecycle::AppLifecyclePlugin,
        asset::AssetPlugin as SimulationAssetPlugin,
        biome::BiomePlugin,
        block::{BlockPlugin, BlockRegistryResource},
        chunk::ChunkLoadingPlugin,
        scheduling::{FixedUpdateSet, RenderPrepSet},
        terrain::TerrainGenerationPlugin,
        time::TimeControlPlugin,
    },
};
use utils::PersistentPaths;

#[instrument(skip_all, fields(name = "main"))]
fn main() {
    attach_logger();

    // setup default bevy app
    let mut app = App::new();

    // resolve platform paths
    let persistent_paths = PersistentPaths::resolve();

    // config of default bevy plugins
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Vantablock".to_string(),
                    resolution: WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: persistent_paths.assets_dir.to_string_lossy().to_string(),
                ..default()
            })
            .disable::<LogPlugin>(),
    );

    // load config & loading trackers into main world
    app.insert_resource(ClientSettings::load_or_create(&persistent_paths));
    app.insert_resource(persistent_paths);
    app.insert_resource(LoadingTracker::default());

    // configure schedule sets
    app.configure_sets(
        FixedUpdate,
        (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain(),
    );

    app.configure_sets(PostUpdate, RenderPrepSet);

    // initialize simulation and renderer
    app.add_plugins((
        // client-specific simulation
        InputModulePlugin,
        PlayerPlugin,
        ShowcasePlugin,
        // shared simulation
        AppLifecyclePlugin,
        SimulationAssetPlugin,
        BiomePlugin,
        BlockPlugin,
        ChunkLoadingPlugin,
        TerrainGenerationPlugin,
        TimeControlPlugin,
        // rendering
        VantablockRenderPlugin,
    ));

    // registry data must be initialized before anything else
    app.add_systems(PreStartup, initialize_simulation_registries_system);

    app.run();
    info!("App exited safely!");
}

/// A system that initializes the simulation registries (textures, blocks, etc.)
/// This is critical for both rendering and simulation logic.
fn initialize_simulation_registries_system(world: &mut World) {
    info!("Initializing simulation registries (textures, blocks)...");
    let client_settings = world.resource::<ClientSettings>().clone();
    let persistent_paths = world.resource::<PersistentPaths>();

    // load textures (CPU-side registry + the stitched texture array image)
    let (texture_array_image, texture_registry) = VoxelTextureProcessor::new(
        persistent_paths.assets_dir.clone(),
        &client_settings.texture_pack,
    )
    .load_and_stitch()
    .unwrap();

    // add the image to Bevy's native Assets<Image>
    let mut image_assets = world.resource_mut::<Assets<Image>>();
    let texture_handle = image_assets.add(texture_array_image);

    // insert resources into world
    world.insert_resource(texture_registry);
    world.insert_resource(BlockTextureArray {
        handle: texture_handle,
    });
    world.init_resource::<BlockRegistryResource>();

    info!("Simulation registries initialized successfully.");
}
