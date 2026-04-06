pub mod input;
pub mod player;
pub mod showcase;

// INFO: -----------------------------
//         Simulation Setup
// -----------------------------------

use crate::render::textures::{BlockTextureArray, load_voxel_texture_assets};
use crate::simulation::{input::InputModulePlugin, player::PlayerPlugin, showcase::ShowcasePlugin};
use bevy::app::{App, FixedUpdate, Plugin, PostUpdate, PreStartup};
use bevy::asset::Assets;
use bevy::prelude::{Image, IntoScheduleConfigs, World, info};
use shared::ecs_core::config::AppConfig;
use shared::simulation::app_lifecycle::AppLifecyclePlugin;
use shared::simulation::asset_management::AssetManagementPlugin;
use shared::simulation::block::BlockRegistryResource;
use shared::simulation::{
    biome::BiomePlugin, block::BlockPlugin, chunk::ChunkLoadingPlugin,
    terrain::TerrainGenerationPlugin, time::TimeControlPlugin,
};
use shared::{FixedUpdateSet, RenderPrepSet};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // configure schedule sets
        app.configure_sets(
            FixedUpdate,
            (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain(),
        );

        app.configure_sets(PostUpdate, RenderPrepSet);

        // add plugins
        app.add_plugins((SharedPlugins, ClientOnlyPlugins));

        // Registry data must be initialized before anything else
        app.add_systems(PreStartup, initialize_simulation_registries_system);
    }
}

fn initialize_simulation_registries_system(world: &mut World) {
    info!("Initializing simulation registries (textures, blocks)...");
    let app_config = world.resource::<AppConfig>().clone();

    // Load textures (CPU-side registry + the stitched texture array image)
    let (texture_array_image, texture_registry) = load_voxel_texture_assets(&app_config).unwrap();

    // Add the image to Bevy's native Assets<Image>
    let mut image_assets = world.resource_mut::<Assets<Image>>();
    let texture_handle = image_assets.add(texture_array_image);

    // Insert resources into world
    world.insert_resource(texture_registry);
    world.insert_resource(BlockTextureArray {
        handle: texture_handle,
    });

    // Now initialize BlockRegistryResource
    world.init_resource::<BlockRegistryResource>();

    info!("Simulation registries initialized successfully.");
}

// INFO: ---------------------------------
//         Plugin Groups (private)
// ---------------------------------------

/// Plugins to run on both the server and client
struct SharedPlugins;
impl Plugin for SharedPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AppLifecyclePlugin,
            AssetManagementPlugin,
            BlockPlugin,
            BiomePlugin,
            ChunkLoadingPlugin,
            TerrainGenerationPlugin,
            TimeControlPlugin,
        ));
    }
}

/// Plugins to run on solely on a client (UI, etc)
struct ClientOnlyPlugins;
impl Plugin for ClientOnlyPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlayerPlugin, InputModulePlugin, ShowcasePlugin));
    }
}
