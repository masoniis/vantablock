use crate::render::chunk::meshing::{OpaqueMeshData, TransparentMeshData};
use bevy::ecs::prelude::Component;
use crossbeam::channel::Receiver;

/// Marks a chunk meshing task in the simulation world that returns a MeshComponent.
#[derive(Component)]
pub struct ChunkMeshingTaskComponent {
    pub receiver: Receiver<(Option<OpaqueMeshData>, Option<TransparentMeshData>)>,
}

/// A signal marking that chunks should be checked for meshing. This check is necessary
/// as chunks require all neighbors to be generated before they mesh.
#[derive(Component)]
pub struct CheckForMeshing;

/// A signal marking that chunks wants to be meshed. In this phase, the chunk is waiting
/// to be assigned to the thread pool, but can't be assigned until all of its relevant
/// neighbors have block data generated.
#[derive(Component)]
pub struct WantsMeshing;
