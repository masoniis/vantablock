use bevy::{
    app::{App, FixedUpdate, PostUpdate},
    asset::Assets,
    log::LogPlugin,
    prelude::{
        default, info, AssetPlugin, ClearColor, Color, Commands, DefaultPlugins, Image,
        IntoScheduleConfigs, PluginGroup, Res, Startup, Window, WindowPlugin, World,
    },
    tasks::AsyncComputeTaskPool,
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
    state::ClientLifecyclePlugin,
    ui::VantablockUiPlugin,
};
use crossbeam::channel::unbounded;
use shared::{
    load::{LoadingTracker, SimulationWorldLoadingTaskComponent, TaskResultCallback},
    simulation::{
        asset::AssetPlugin as SimulationAssetPlugin,
        biome::BiomePlugin,
        block::{BlockPlugin, BlockRegistryResource},
        chunk::ChunkLoadingPlugin,
        lifecycle::SimulationLifecyclePlugin,
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

    // red clear color to prevent white screen flash
    app.insert_resource(ClearColor(Color::linear_rgb(1.0, 0.0, 0.0)));

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
        ClientLifecyclePlugin,
        InputModulePlugin,
        PlayerPlugin,
        ShowcasePlugin,
        // shared simulation
        SimulationLifecyclePlugin,
        SimulationAssetPlugin,
        BiomePlugin,
        BlockPlugin,
        ChunkLoadingPlugin,
        TerrainGenerationPlugin,
        TimeControlPlugin,
        // rendering
        VantablockUiPlugin,
        VantablockRenderPlugin,
    ));

    // start background registry initialization
    app.add_systems(Startup, start_async_registry_initialization);

    app.run();
    info!("App exited safely!");
}

/// A system that starts the asynchronous initialization of simulation registries.
/// This prevents the main thread from blocking during heavy tasks like texture stitching and block registry generation.
fn start_async_registry_initialization(
    mut commands: Commands,
    client_settings: Res<ClientSettings>,
    persistent_paths: Res<PersistentPaths>,
) {
    info!("Starting asynchronous simulation registry initialization...");

    let (sender, receiver) = unbounded();
    let settings = client_settings.clone();
    let paths = persistent_paths.clone();

    AsyncComputeTaskPool::get()
        .spawn(async move {
            info!("Initializing simulation registries in background...");

            // 1. Texture Stitching (CPU Intensive)
            let (texture_array_image, texture_registry) =
                VoxelTextureProcessor::new(paths.assets_dir.clone(), &settings.texture_pack)
                    .load_and_stitch()
                    .expect("Failed to load and stitch textures");

            // 2. Block Registry Generation (CPU Intensive, depends on texture registry)
            let block_registry =
                BlockRegistryResource::load_from_disk(Some(&texture_registry), &paths);

            // prepare callback to apply results on main thread
            let callback: TaskResultCallback = Box::new(move |commands: &mut Commands| {
                info!("Applying simulation registry results to the world.");

                // access world via commands to insert resources
                commands.queue(move |world: &mut World| {
                    let mut image_assets = world.resource_mut::<Assets<Image>>();
                    let texture_handle = image_assets.add(texture_array_image);

                    // insert both registries
                    world.insert_resource(texture_registry);
                    world.insert_resource(block_registry);

                    // insert texture array handle
                    world.insert_resource(BlockTextureArray {
                        handle: texture_handle,
                    });
                });

                info!("Simulation registries initialized successfully.");
            });

            sender
                .send(callback)
                .expect("Failed to send registry task result");
        })
        .detach();

    // register this as a loading task so the game waits for it
    commands.spawn(SimulationWorldLoadingTaskComponent { receiver });
}
