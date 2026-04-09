pub mod pipeline;

pub use pipeline::*;

use bevy::app::{App, Plugin};
use bevy::render::render_resource::SpecializedRenderPipelines;

pub struct SkyboxRenderPassPlugin;

impl Plugin for SkyboxRenderPassPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        app.init_resource::<SkyboxPipeline>();
        app.init_resource::<SpecializedRenderPipelines<SkyboxPipeline>>();
    }
}
