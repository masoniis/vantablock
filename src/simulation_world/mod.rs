pub mod app_lifecycle;
pub mod asset_management;
pub mod biome;
pub mod block;
pub mod chunk;
pub mod input;
pub mod player;
pub mod scheduling;
pub mod showcase;
pub mod terrain;
pub mod time;
pub mod user_interface;

pub use scheduling::{FixedUpdateSet, SimulationSet, StartupSet};

// INFO: -----------------------------
//         Simulation Setup
// -----------------------------------

use crate::render_world::{
    global_extract::utils::initialize_simulation_world_for_extract,
    textures::TextureRegistryResource,
};
use crate::simulation_world::app_lifecycle::AppLifecyclePlugin;
use crate::simulation_world::{
    asset_management::AssetManagementPlugin,
    biome::BiomePlugin,
    block::BlockPlugin,
    chunk::ChunkLoadingPlugin,
    input::{InputModulePlugin, WindowSizeResource},
    player::PlayerPlugin,
    showcase::ShowcasePlugin,
    terrain::TerrainGenerationPlugin,
    time::TimeControlPlugin,
    user_interface::UiPlugin,
};
use bevy::app::{App, FixedUpdate, Plugin, Startup, Update};
use bevy::prelude::IntoScheduleConfigs;
use winit::window::Window;

use crate::ecs_core::worlds::SimulationWorldMarker;

/// Creates and configures a new simulation app.
pub fn setup_simulation_app(
    window: &Window,
    texture_registry_resource: TextureRegistryResource,
) -> App {
    let mut app = App::new();

    // add resources built from the app
    app.insert_resource(WindowSizeResource::new(window.inner_size()))
        .insert_resource(texture_registry_resource);

    // configure schedule sets before adding plugins
    app.configure_sets(
        Startup,
        (StartupSet::ResourceInitialization, StartupSet::Tasks).chain(),
    );

    app.configure_sets(
        FixedUpdate,
        (FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain(),
    );

    app.configure_sets(
        Update,
        (
            SimulationSet::Input,
            SimulationSet::PreUpdate,
            SimulationSet::Update,
            SimulationSet::Physics,
            SimulationSet::PostUpdate,
            SimulationSet::RenderPrep,
        )
            .chain(),
    );

    // now add plugins, which can safely use the configured sets
    app.add_plugins((SharedPlugins, ClientOnlyPlugins));

    initialize_simulation_world_for_extract(app.world_mut());
    app.world_mut().insert_resource(SimulationWorldMarker);

    app
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
        app.add_plugins((PlayerPlugin, UiPlugin, InputModulePlugin, ShowcasePlugin));
    }
}
