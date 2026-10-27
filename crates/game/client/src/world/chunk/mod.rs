//! The client's chunk data: which chunks it holds and what state they are in
//! (`ClientChunkManager`), streaming them from the server by distance (`loading`), and turning
//! received bytes into chunk entities (`decompression`).
//!
//! Nothing here touches rendering, so it also runs in headless clients. Meshing and GPU upload
//! live in `render::chunk` and build on this module.

pub mod decompression;
pub mod loading;
pub mod manager;
pub mod range;

pub use manager::{ClientChunkManager, ClientChunkState, NeighborInfo};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::prelude::*;
use common::FixedUpdateSet;
use decompression::{
    DecompressedChunkMessage, apply_decompressed_chunk_data_system, decompress_chunk_data_system,
    poll_decompression_tasks_system,
};
use loading::{manage_distance_based_chunk_loading_targets_system, unload_all_chunks_system};
use range::active_camera_changed_chunk;

/// Owns the client's chunk state and streams chunk data from the server.
pub struct ClientChunkPlugin;

impl Plugin for ClientChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientChunkManager>();
        app.add_message::<DecompressedChunkMessage>();

        app.add_systems(
            PreUpdate,
            manage_distance_based_chunk_loading_targets_system.run_if(active_camera_changed_chunk),
        );

        app.add_systems(
            Update,
            (
                decompress_chunk_data_system,
                poll_decompression_tasks_system,
            ),
        );
        app.add_systems(
            FixedUpdate,
            apply_decompressed_chunk_data_system.in_set(FixedUpdateSet::MainLogic),
        );

        // the session's chunks belong to its server, so drop them when leaving the game
        app.add_systems(
            OnExit(ClientLifecycleState::InGame),
            unload_all_chunks_system,
        );
    }
}
