pub mod global_extract;
pub mod graphics_context;
pub mod passes;
pub mod scheduling;
pub mod textures;
pub mod types;

use crate::render_world::textures::StagingTextureImages;
pub use scheduling::{RenderSchedule, RenderSet};

// INFO: --------------------------------
//         render world interface
// --------------------------------------

use crate::ecs_core::{
    state_machine::{self, AppState, GameState, StatePlugin},
    worlds::RenderWorldMarker,
};
use crate::prelude::*;
use crate::render_world::{
    global_extract::{
        RenderCameraResource, RenderMeshStorageResource, RenderTimeResource,
        SimulationExtractionPlugin,
    },
    graphics_context::{GraphicsContext, GraphicsContextPlugin},
    passes::{core::setup_render_graph, RenderPassManagerPlugin},
};
use bevy::ecs::prelude::*;
use std::ops::{Deref, DerefMut};

pub struct RenderWorldInterface {
    pub common: CommonEcsInterface,
}

impl Deref for RenderWorldInterface {
    type Target = CommonEcsInterface;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl DerefMut for RenderWorldInterface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl RenderWorldInterface {
    /// Creates a new render world with a sane default configuration
    pub fn new(
        graphics_context: GraphicsContext,
        staging_texture_images: StagingTextureImages,
    ) -> Self {
        let mut builder = EcsBuilder::new();

        // INFO: -----------------------------------------------------
        //         set up graphics-context dependent resources
        // -----------------------------------------------------------

        // Setup render graph runs as an early system since it needs mutable world access
        setup_render_graph(&mut builder.world);

        // Add any resources that require specific app input
        builder
            .add_resource(staging_texture_images)
            .add_resource(RenderWorldMarker);

        // INFO: --------------------------------
        //         non-mod specific setup
        // --------------------------------------

        builder
            .schedules
            .entry(RenderSchedule::Main)
            .configure_sets(
                (
                    RenderSet::StateUpdate,
                    RenderSet::Prepare,
                    RenderSet::Queue,
                    RenderSet::Render,
                )
                    .chain(),
            );

        // Resources for rendering
        builder
            .init_resource::<RenderTimeResource>()
            .init_resource::<RenderCameraResource>()
            .init_resource::<RenderMeshStorageResource>();

        // Specifically implemented plugins
        builder
            .add_plugin(GraphicsContextPlugin::new(graphics_context))
            .add_plugin(RenderPassManagerPlugin)
            .add_plugin(SimulationExtractionPlugin);
        // Generic auto-constructed plugins
        builder
            .add_plugin(StatePlugin::<AppState>::default())
            .add_plugin(StatePlugin::<GameState>::default());

        builder.schedule_entry(RenderSchedule::Main).add_systems(
            (
                // these are applied by state changes detected in extraction
                state_machine::apply_state_transition_system::<AppState>,
                state_machine::apply_state_transition_system::<GameState>,
            )
                .in_set(RenderSet::StateUpdate),
        );

        Self::build_render_world(builder)
    }

    /// Builds the final state and returns the final render world interface.
    fn build_render_world(mut builder: EcsBuilder) -> RenderWorldInterface {
        for (_, schedule) in builder.schedules.drain_schedules() {
            builder.world.add_schedule(schedule);
        }

        RenderWorldInterface {
            common: CommonEcsInterface {
                world: builder.world,
            },
        }
    }
}
