#[allow(clippy::module_inception)]
pub mod graphics_context;
pub mod reconfigure_surface;
pub mod resources;

use bevy::ecs::prelude::*;
pub use graphics_context::GraphicsContext;
pub use reconfigure_surface::reconfigure_wgpu_surface_system;

// INFO: ---------------------------
//         Plugin definition
// ---------------------------------

use crate::{
    ecs_core::{EcsBuilder, Plugin},
    render_world::{
        global_extract::RenderWindowSizeResource,
        graphics_context::resources::{
            RenderAdapter, RenderDevice, RenderQueue, RenderSurface, RenderSurfaceConfig,
        },
        scheduling::RenderSchedule,
    },
};

pub struct GraphicsContextPlugin {
    context: GraphicsContext,
}

/// GraphicsContextPlugin is unique in that it needs to be passed the context
/// created by the outer app loop. It can then create resources based on this
/// for the rest of the ECS world to employ.
impl GraphicsContextPlugin {
    /// Creates a new plugin, taking ownership of the app's graphics context.
    pub fn new(context: GraphicsContext) -> Self {
        Self { context }
    }
}

impl Plugin for GraphicsContextPlugin {
    fn build(&self, builder: &mut EcsBuilder) {
        builder
            .add_resource(RenderDevice(self.context.device.clone()))
            .add_resource(RenderQueue(self.context.queue.clone()))
            .add_resource(RenderAdapter(self.context.adapter.clone()))
            .add_resource(RenderSurface(self.context.surface.clone()))
            .add_resource(RenderSurfaceConfig(self.context.config.clone()));

        // INFO: -----------------
        //         Startup
        // -----------------------

        // INFO: -----------------
        //         Extract
        // -----------------------

        // INFO: -----------------
        //         Prepare
        // -----------------------

        // INFO: ---------------
        //         Queue
        // ---------------------

        builder.schedule_entry(RenderSchedule::Main).add_systems(
            // this isn't in a set because it doesnt really matter when it runs due to the fact
            // that RenderWindowSizeResource gets updated from extraction, which runs before main.
            reconfigure_wgpu_surface_system
                .run_if(resource_changed_or_removed::<RenderWindowSizeResource>),
        );
    }
}
