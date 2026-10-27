pub mod gpu_resources;
pub mod queue;
pub mod render;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::render::passes::bounding_box::{
    queue::queue_wireframe_system, render::bounding_box_pass,
};
use bevy::{
    core_pipeline::{Core3d, Core3dSystems, core_3d::main_transparent_pass_3d},
    prelude::{App, IntoScheduleConfigs, Plugin},
    render::{
        render_resource::SpecializedRenderPipelines,
        {Render, RenderSystems},
    },
};
use gpu_resources::{
    UnitCubeMesh, WireframeObjectBuffer, WireframePipeline,
    object_binding::WireframeObjectBindGroupLayout,
};

pub struct WireframeRenderPassPlugin;

impl Plugin for WireframeRenderPassPlugin {
    fn build(&self, app: &mut App) {
        // queue
        app.add_systems(Render, queue_wireframe_system.in_set(RenderSystems::Queue));

        // camera schedule wiring, drawn over everything else
        app.add_systems(
            Core3d,
            bounding_box_pass
                .after(main_transparent_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }

    fn finish(&self, app: &mut App) {
        // INFO: -----------------
        //         startup
        // -----------------------

        app.init_resource::<WireframeObjectBindGroupLayout>();
        app.init_resource::<WireframeObjectBuffer>();
        app.init_resource::<WireframePipeline>();
        app.init_resource::<SpecializedRenderPipelines<WireframePipeline>>();
        app.init_resource::<UnitCubeMesh>();
    }
}
