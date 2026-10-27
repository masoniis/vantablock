pub mod mesh;
pub mod render;
pub mod skybox;

pub use mesh::*;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::passes::opaque::{
    mesh::pipeline::Opaque3dPipeline, render::opaque_pass, skybox::OpaqueSkyboxPipeline,
};
use bevy::{
    core_pipeline::{Core3d, Core3dSystems, core_3d::main_opaque_pass_3d},
    prelude::{App, IntoScheduleConfigs, Plugin},
    render::{
        render_resource::SpecializedRenderPipelines,
        {Render, RenderSystems},
    },
};

pub struct OpaqueRenderPassPlugin;

impl Plugin for OpaqueRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // INFO: ---------------
        //         queue
        // ---------------------

        app.add_systems(
            Render,
            queue::queue_opaque_system.in_set(RenderSystems::Queue),
        );

        // INFO: ---------------------------------
        //         camera schedule wiring
        // ---------------------------------------

        app.add_systems(
            Core3d,
            opaque_pass
                .after(main_opaque_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         startup
        // -----------------------

        app.init_resource::<Opaque3dPipeline>();
        app.init_resource::<SpecializedRenderPipelines<Opaque3dPipeline>>();
        app.init_resource::<OpaqueSkyboxPipeline>();
        app.init_resource::<SpecializedRenderPipelines<OpaqueSkyboxPipeline>>();
    }
}
