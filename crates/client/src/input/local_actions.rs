use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;

/// Actions that are local to the client and do not affect the server's state.
/// These are typically used for UI toggles, debug views, and local camera controls.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum LocalClientAction {
    ToggleDiagnostics,
    ToggleDebugMenu,
    ToggleOpaqueWireframeMode,
    ToggleChunkBorders,
    TogglePause, // Assuming this is an escape menu, not pausing the actual multiplayer server!

    // Showcase actions
    Showcase0,
    Showcase1,
    Showcase2,
    Showcase3,
    Showcase4,
    Showcase5,
    Showcase6,
    Showcase7,
    Showcase8,
    Showcase9,
}
