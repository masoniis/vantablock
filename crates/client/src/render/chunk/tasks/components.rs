use crate::render::chunk::meshing::{OpaqueMeshData, TransparentMeshData};
use bevy::ecs::prelude::Component;
use bevy::tasks::Task;
use crossbeam::channel::Receiver;
use shared::world::chunk::ChunkCoord;

/// Marks a chunk meshing task in the simulation world that returns a MeshComponent.
#[derive(Component)]
pub struct ChunkMeshingTaskComponent {
    pub receiver: Receiver<(Option<OpaqueMeshData>, Option<TransparentMeshData>)>,
}

/// Marks a chunk decompression task.
#[derive(Component)]
pub struct DecompressionTask(pub Task<(ChunkCoord, Vec<u8>)>);

/// A signal marking that chunks should be checked for meshing. This check is necessary
/// as chunks require all neighbors to be generated before they mesh.
#[derive(Component)]
pub struct CheckForMeshing;

/// A signal marking that chunks wants to be meshed. In this phase, the chunk is waiting
/// to be assigned to the thread pool, but can't be assigned until all of its relevant
/// neighbors have block data generated.
#[derive(Component)]
pub struct WantsMeshing;
