use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;

/// Actions that are local to the client and do not affect the server's state or the player/character.
///
/// These are typically used for UI toggles, debug views, etc.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum ClientAction {
    // continuous inputs
    #[actionlike(DualAxis)]
    Look,

    // random stuff
    ToggleDiagnostics,
    ToggleDebugMenu,
    ToggleOpaqueWireframeMode,
    ToggleChunkBorders,
    TogglePause,

    // showcase actions
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
