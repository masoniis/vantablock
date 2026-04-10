use crate::simulation::chunk::ChunkCoord;
use crate::simulation::input::types::SimulationAction;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// A discrete input action performed by the player.
    Action(SimulationAction),
    /// A request for the server to send voxel data for a specific chunk.
    RequestChunk(ChunkCoord),
    /// Updates the server on the player's current view orientation/camera state.
    /// Necessary for authoritative targeting or raycasting calculations.
    UpdateView { forward: Vec3 },
}
