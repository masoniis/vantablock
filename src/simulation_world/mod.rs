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

pub use scheduling::{
    FixedUpdateSet, OnEnter, OnExit, SimulationSchedule, SimulationSet, StartupSet,
};

// INFO: -----------------------------
//         Sim world interface
// -----------------------------------

use crate::render_world::{
    global_extract::utils::initialize_simulation_world_for_extract,
    textures::TextureRegistryResource,
};
use crate::simulation_world::{
    asset_management::{AssetManagementPlugin, MeshAsset},
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
use crate::{
    ecs_core::{worlds::SimulationWorldMarker, CommonEcsInterface, EcsBuilder, PluginGroup},
    simulation_world::app_lifecycle::AppLifecyclePlugin,
};
use bevy::ecs::prelude::*;
use std::ops::{Deref, DerefMut};
use winit::window::Window;

pub struct SimulationWorldInterface {
    pub common: CommonEcsInterface,
}

impl SimulationWorldInterface {
    pub fn send_event<E: Message>(&mut self, event: E) {
        self.world.write_message(event);
    }
}

impl Deref for SimulationWorldInterface {
    type Target = CommonEcsInterface;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl DerefMut for SimulationWorldInterface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl SimulationWorldInterface {
    pub fn new(window: &Window, texture_registry_resource: TextureRegistryResource) -> Self {
        let mut builder = EcsBuilder::new();

        // add resources built from the app
        builder
            .add_resource(WindowSizeResource::new(window.inner_size()))
            .add_resource(texture_registry_resource);

        // configure schedule sets before adding plugins
        builder
            .schedules
            .entry(SimulationSchedule::Startup)
            .configure_sets((StartupSet::ResourceInitialization, StartupSet::Tasks).chain());

        builder
            .schedules
            .entry(SimulationSchedule::FixedUpdate)
            .configure_sets((FixedUpdateSet::PreUpdate, FixedUpdateSet::MainLogic).chain());

        builder
            .schedules
            .entry(SimulationSchedule::Main)
            .configure_sets(
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
        builder
            .add_plugins(SharedPlugins)
            .add_plugins(ClientOnlyPlugins);

        Self::build_simulation_world(builder)
    }

    fn build_simulation_world(mut builder: EcsBuilder) -> SimulationWorldInterface {
        for (_, schedule) in builder.schedules.drain_schedules() {
            builder.world.add_schedule(schedule);
        }

        let mut interface = SimulationWorldInterface {
            common: CommonEcsInterface {
                world: builder.world,
            },
        };

        initialize_simulation_world_for_extract(&mut interface.world);
        interface.world.insert_resource(SimulationWorldMarker);

        interface
    }
}

// INFO: ---------------------------------
//         Plugin Groups (private)
// ---------------------------------------

/// Plugins to run on both the server and client
struct SharedPlugins;
impl PluginGroup for SharedPlugins {
    fn build(self, builder: &mut EcsBuilder) {
        builder
            .add_plugin(AppLifecyclePlugin)
            .add_plugin(AssetManagementPlugin)
            .add_plugin(BlockPlugin)
            .add_plugin(BiomePlugin)
            .add_plugin(ChunkLoadingPlugin)
            .add_plugin(TerrainGenerationPlugin)
            .add_plugin(TimeControlPlugin);
    }
}

/// Plugins to run on solely on a client (UI, etc)
struct ClientOnlyPlugins;
impl PluginGroup for ClientOnlyPlugins {
    fn build(self, builder: &mut EcsBuilder) {
        builder
            .add_plugin(PlayerPlugin)
            .add_plugin(UiPlugin)
            .add_plugin(InputModulePlugin)
            .add_plugin(ShowcasePlugin);
    }
}
