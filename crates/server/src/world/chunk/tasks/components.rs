use crate::world::chunk::components::GeneratedChunkComponentBundle;
use bevy::ecs::prelude::Component;
use crossbeam::channel::Receiver;

/// Marks a chunk loading task in the simulation world that returns nothing.
#[derive(Component)]
pub struct ChunkGenerationTaskComponent {
    pub receiver: Receiver<GeneratedChunkComponentBundle>,
}
