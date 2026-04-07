pub mod data;
pub mod pipeline;
pub mod scheduling;
pub mod texture;
pub mod types;

// INFO: --------------------------------
//         render world interface
// --------------------------------------

use crate::prelude::*;
use crate::render::{
    data::{
        ExtractedSun, MeshesToUploadQueue, RenderMeshStorageResource, RenderTimeResource,
        SimulationExtractionPlugin,
    },
    pipeline::{
        RenderGraphEdgesPlugin, WorldRenderPassesPlugin,
        main_passes::{
            bounding_box_pass::extract::WireframeToggleState,
            opaque_pass::{extract::extract_opaque_meshes, startup::OpaqueRenderMode},
            transparent_pass::extract::extract_transparent_meshes,
        },
    },
    texture::BlockTextureArray,
};
use bevy::{
    app::{App, Plugin, SubApp},
    asset::AssetApp,
    prelude::{Add, Commands, On},
    render::{
        ExtractSchedule, RenderApp, extract_resource::ExtractResourcePlugin,
        sync_world::SyncToRenderWorld,
    },
};
use shared::simulation::{
    asset::VoxelMeshAsset,
    block::TargetedBlock,
    chunk::{OpaqueMeshComponent, TransparentMeshComponent},
};

/// Plugin responsible for attaching our custom render logic to Bevy's native RenderApp
pub struct VantablockRenderPlugin;

impl Plugin for VantablockRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pipeline::shader_registry::VantablockShaderPlugin);

        app.init_asset::<VoxelMeshAsset>();

        // register extraction plugins on the main app
        app.add_plugins((
            // resources
            ExtractResourcePlugin::<ExtractedSun>::default(),
            ExtractResourcePlugin::<RenderTimeResource>::default(),
            ExtractResourcePlugin::<OpaqueRenderMode>::default(),
            ExtractResourcePlugin::<TargetedBlock>::default(),
        ));

        app.add_plugins((
            ExtractResourcePlugin::<WireframeToggleState>::default(),
            ExtractResourcePlugin::<BlockTextureArray>::default(),
        ));

        // INFO: ----------------------------------------------------------------
        //         Main World Synchronization
        // ----------------------------------------------------------------------

        // Register observers to mark mesh entities for synchronization to the render world
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
    // Resources for rendering
    sub_app
        .init_resource::<RenderTimeResource>()
        .init_resource::<RenderMeshStorageResource>()
        .init_resource::<MeshesToUploadQueue>();

    // Specifically implemented plugins (These run strictly in the Render World)
    sub_app.add_plugins((
        WorldRenderPassesPlugin,
        SimulationExtractionPlugin,
        RenderGraphEdgesPlugin,
    ));
}
