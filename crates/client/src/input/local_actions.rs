use bevy::prelude::Reflect;
use leafwing_input_manager::Actionlike;

/// Actions that are local to the client and do not affect the server's state or the player/character.
///
/// These are typically used for UI toggles, debug views, etc. They live on a global
/// `ActionState<ClientAction>` resource.
#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum ClientAction {
    // continuous inputs
    #[actionlike(Axis)]
    Zoom,

    // menus
    TogglePause,
    ToggleDebugMenu,

    // renderer inspection
    CycleDebugView,
    ToggleSceneAcceleration,
    CycleRenderScale,

    // world clock requests (sent to the server)
    TimeForward,
    TimeBackward,
    ToggleDayCycle,
}
