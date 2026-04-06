use bevy::ecs::prelude::Resource;
use bevy::render::extract_resource::ExtractResource;
use shared::simulation::time::FrameClock;

#[derive(Resource, Debug, Default, Clone)]
pub struct RenderTimeResource {
    pub total_elapsed_seconds: f32,
}

impl ExtractResource for RenderTimeResource {
    type Source = FrameClock;

    fn extract_resource(source: &Self::Source) -> Self {
        RenderTimeResource {
            total_elapsed_seconds: source.elapsed.as_secs_f32(),
        }
    }
}
