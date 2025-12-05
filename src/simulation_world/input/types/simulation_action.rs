/// Defines
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SimulationAction {
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
    Showcase0, // personal reset to flat
}
