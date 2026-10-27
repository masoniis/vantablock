use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;

/// Actions that are local to the client and do not affect the server's state or the player/character.
///
/// These are typically used for UI toggles, debug views, etc. Their `ActionState<ClientAction>`
/// lives on a single app-lifetime entity, read with `Single`.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum ClientAction {
    // continuous inputs
    #[actionlike(Axis)]
    Zoom,

    // random stuff
    ToggleDiagnostics,
    ToggleDebugMenu,
    ToggleOpaqueWireframeMode,
    ToggleChunkBorders,
    TogglePause,
}
