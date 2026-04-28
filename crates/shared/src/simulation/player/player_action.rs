use leafwing_input_manager::Actionlike;
use bevy::prelude::Reflect;
use bevy::reflect::{ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, Serialize, Deserialize)]
#[reflect(PartialEq, Hash, Serialize, Deserialize)]
pub enum PlayerAction {
    // Core player movement
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveFaster,

    // Core player interaction
    BreakVoxel,
    PlaceVoxel,

    // Terrain interactions
    CycleActiveTerrainGenerator,

    // Time control interactions
    JumpGameTimeForward,
    JumpGameTimeBackward,
    PauseGameTime,

    // Misc
    ToggleDiagnostics,
    ToggleOpaqueWireframeMode,
    ToggleChunkBorders,
    TogglePause,

    // Showcase actions
    Showcase1,
    Showcase2,
    Showcase3,
    Showcase4,
    Showcase5,
    Showcase6,
    Showcase7,
    Showcase8,
    Showcase9,
    Showcase0,
}
