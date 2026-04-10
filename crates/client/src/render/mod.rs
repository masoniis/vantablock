pub mod block;
pub mod chunk;
pub mod data;
pub mod passes;
pub mod resources;
pub mod scheduling;
pub mod shaders;
pub mod texture;
pub mod types;

pub use data::*;

// INFO: --------------------------------
//         render world interface
// --------------------------------------

use crate::input::systems::toggle_chunk_borders::ChunkBoundsToggle;
use crate::input::systems::toggle_opaque_wireframe::OpaqueRenderMode;
use crate::prelude::*;
use crate::render::{
    chunk::{OpaqueMeshComponent, TransparentMeshComponent},
    passes::{
        opaque::extract::extract_opaque_meshes, transparent::extract::extract_transparent_meshes,
        RenderGraphEdgesPlugin, WorldRenderPassesPlugin,
    },
    texture::BlockTextureArray,
};
use bevy::{
    app::{App, Plugin, SubApp},
    prelude::{Add, Commands, On},
    render::{
        extract_resource::ExtractResourcePlugin, sync_world::SyncToRenderWorld, ExtractSchedule,
        RenderApp,
    },
};
use shared::simulation::block::TargetedBlock;

/// Plugin responsible for attaching custom render logic to Bevy's native RenderApp
pub struct VantablockRenderPlugin;

impl Plugin for VantablockRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(shaders::VantablockShaderPlugin);
        app.add_plugins(block::BlockRenderPlugin);
        app.add_plugins(chunk::ChunkMeshingPlugin);

        // register extraction plugins on the main app
        app.add_plugins((
            // resources
            ExtractResourcePlugin::<ExtractedSun>::default(),
            ExtractResourcePlugin::<RenderTimeResource>::default(),
            ExtractResourcePlugin::<OpaqueRenderMode>::default(),
            ExtractResourcePlugin::<TargetedBlock>::default(),
            ExtractResourcePlugin::<BlockTextureArray>::default(),
        ));

        // INFO: ------------------------------------
        //         main world synchronization
        // ------------------------------------------

        // register observers to mark mesh entities for synchronization to the render world
        app.add_observer(
            |add: On<Add, OpaqueMeshComponent>, mut commands: Commands| {
                commands.entity(add.entity).insert(SyncToRenderWorld);
            },
        );

        app.add_observer(
            |add: On<Add, TransparentMeshComponent>, mut commands: Commands| {
                commands.entity(add.entity).insert(SyncToRenderWorld);
            },
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("RenderApp not found! Ensure DefaultPlugins are added before this plugin.");
            return;
        };

        // manual component extraction MUST be added to the render app
        render_app.add_systems(
            ExtractSchedule,
            (extract_opaque_meshes, extract_transparent_meshes),
        );

        pre_setup_render_sub_app(render_app);
    }
}

/// Configures a sub-app with its base configuration, before graphics context is ready.
pub fn pre_setup_render_sub_app(sub_app: &mut SubApp) {
    // resources for rendering
    sub_app
        .init_resource::<ChunkBoundsToggle>()
        .init_resource::<RenderTimeResource>()
        .init_resource::<RenderMeshStorageResource>()
        .init_resource::<MeshesToUploadQueue>();

    // specifically implemented plugins (these run strictly in the Render World)
    sub_app.add_plugins((
        WorldRenderPassesPlugin,
        SimulationExtractionPlugin,
        RenderGraphEdgesPlugin,
    ));
}
