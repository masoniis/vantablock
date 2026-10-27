pub mod extract;
pub mod prepare;
pub mod queue;
pub mod render;
pub mod startup;

use startup::TransparentPipeline;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::passes::{
    opaque::render::opaque_pass,
    transparent::{
        prepare::prepare_transparent_meshes_system, queue::queue_and_prepare_transparent_system,
        render::transparent_pass,
    },
};
use bevy::prelude::IntoScheduleConfigs;
use bevy::{
    app::{App, Plugin},
    core_pipeline::{Core3d, Core3dSystems, core_3d::main_transparent_pass_3d},
    render::{Render, RenderSystems, render_resource::SpecializedRenderPipelines},
};

pub struct TransparentRenderPassPlugin;

impl Plugin for TransparentRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: -----------------
        //         prepare
        // -----------------------

        app.add_systems(
            Render,
            prepare_transparent_meshes_system.in_set(RenderSystems::Prepare),
        );

        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(
            Render,
            queue_and_prepare_transparent_system.in_set(RenderSystems::Queue),
        );

        // INFO: ---------------------------------
        //         camera schedule wiring
        // ---------------------------------------

        // after the opaque pass (and so Bevy's), before Bevy's own transparent pass
        app.add_systems(
            Core3d,
            transparent_pass
                .after(opaque_pass)
                .before(main_transparent_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         startup
        // -----------------------

        app.init_resource::<TransparentPipeline>();
        app.init_resource::<SpecializedRenderPipelines<TransparentPipeline>>();
    }
}
