use bevy::ecs::prelude::Component;

/// Marks a chunk whose blocks changed, or whose neighbour's blocks along their shared border
/// changed. Anything built from the blocks around a chunk, like its mesh, is rebuilt.
#[derive(Component)]
pub struct ChunkEdited;
