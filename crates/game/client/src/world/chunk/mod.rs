//! The client's chunk data: which chunks it holds and what state they are in
//! (`ClientChunkMap`), streaming them from the server by distance (`loading`), and turning
//! received bytes into chunk entities (`decompression`).
//!
//! Nothing here touches rendering, so it also runs in headless clients. Meshing and GPU upload
//! live in `render::chunk` and build on this module.

pub mod chunk_map;
pub mod decompression;
pub mod edits;
pub mod loading;
pub mod range;

pub use chunk_map::{ClientChunkMap, ClientChunkState};
pub use edits::ChunkEdited;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::ClientLifecycleState;
use bevy::prelude::*;
use common::world::chunk::update_chunk_coords_system;
use decompression::{
    DecompressedChunkMessage, apply_decompressed_chunk_data_system, decompress_chunk_data_system,
    poll_decompression_tasks_system,
};
use loading::{manage_distance_based_chunk_loading_targets_system, unload_all_chunks_system};
use range::streaming_anchor_changed_chunk;

/// Owns the client's chunk state and streams chunk data from the server.
pub struct ClientChunkPlugin;

impl Plugin for ClientChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientChunkMap>();
        app.add_message::<DecompressedChunkMessage>();

        app.add_systems(
            PreUpdate,
            manage_distance_based_chunk_loading_targets_system
                // the anchor's chunk must already follow its position this frame
                .after(update_chunk_coords_system)
                .run_if(streaming_anchor_changed_chunk),
        );

        app.add_systems(
            Update,
            (
                decompress_chunk_data_system,
                poll_decompression_tasks_system,
                apply_decompressed_chunk_data_system,
            )
                .chain(),
        );

        // the session's chunks belong to its server, so drop them when leaving the game
        app.add_systems(
            OnExit(ClientLifecycleState::InGame),
            unload_all_chunks_system,
        );
    }
}
