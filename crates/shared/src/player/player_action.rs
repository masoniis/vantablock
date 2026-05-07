use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;
use serde::{Deserialize, Serialize};

/// Actions that are sent over the network from the client to the server.
/// These actions represent the authoritative state changes requested by the player.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Move the player forward.
    MoveForward,
    /// Move the player backward.
    MoveBackward,
    /// Move the player to the left.
    MoveLeft,
    /// Move the player to the right.
    MoveRight,
    /// Sprint or move at a faster pace.
    MoveFaster,

    /// Primary interaction intent (e.g., breaking a block).
    PrimaryInteract,
    /// Secondary interaction intent (e.g., placing a block).
    SecondaryInteract,

    /// Request to jump the game time forward.
    JumpGameTimeForward,
    /// Request to jump the game time backward.
    JumpGameTimeBackward,
    /// Request to toggle game time pausing.
    PauseGameTime,
}
