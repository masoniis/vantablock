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

use crate::ecs_core::worlds::RenderWorldMarker;
use crate::render_world::{
    global_extract::{
        RenderMeshStorageResource, RenderTimeResource, RenderWindowSizeResource,
        SimulationExtractionPlugin,
    },
    graphics_context::{GraphicsContext, GraphicsContextPlugin},
    passes::{RenderPassManagerPlugin, core::setup_render_graph},
};
use bevy::app::SubApp;
use bevy::prelude::IntoScheduleConfigs;

/// Configures a sub-app with a sane default configuration for rendering.
pub fn setup_render_sub_app(
    sub_app: &mut SubApp,
    graphics_context: GraphicsContext,
    staging_texture_images: StagingTextureImages,
) {
    // INFO: -----------------------------------------------------
    //         set up graphics-context dependent resources
    // -----------------------------------------------------------

    // Setup render graph runs as an early system since it needs mutable world access
    setup_render_graph(sub_app.world_mut());

    // Add any resources that require specific app input
    sub_app
        .insert_resource(staging_texture_images)
        .insert_resource(RenderWorldMarker);

    // INFO: --------------------------------
    //         non-mod specific setup
    // --------------------------------------

    sub_app.configure_sets(
        RenderSchedule::Main,
        (RenderSet::Prepare, RenderSet::Queue, RenderSet::Render).chain(),
    );

    // Resources for rendering
    sub_app
        .init_resource::<RenderTimeResource>()
        .init_resource::<RenderWindowSizeResource>()
        .init_resource::<RenderMeshStorageResource>();

    // Specifically implemented plugins
    sub_app.add_plugins((
        GraphicsContextPlugin::new(graphics_context),
        RenderPassManagerPlugin,
        SimulationExtractionPlugin,
    ));
}
