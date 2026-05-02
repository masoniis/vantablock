use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;
use serde::{Deserialize, Serialize};

/// Actions that are sent over the network from the client to the server.
/// These actions represent the authoritative state changes requested by the player.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum PlayerAction {
    // Movement
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveFaster,

    // World Interaction
    BreakBlock,
    PlaceBlock,

    // Server-side commands
    CycleActiveTerrainGenerator,
    JumpGameTimeForward,
    JumpGameTimeBackward,
    PauseGameTime,
}
