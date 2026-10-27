use bevy::ecs::prelude::SystemSet;

/// Systems that prepare data for the render world.
/// These typically run at the very end of Bevy's native `PostUpdate` schedule.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct RenderPrepSet;
