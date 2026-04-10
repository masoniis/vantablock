use crate::state::enums::AppState;
use bevy::prelude::{StateSet, SubStates};

/// An enum representing the networking mode of the server module.
#[derive(SubStates, Default, Debug, Clone, Eq, PartialEq, Hash)]
#[source(AppState = AppState::Running)]
pub enum NetworkingMode {
    #[default]
    /// The networking module is inactive. This may be, for example, the case on a client
    /// which is in the main menu.
    Inactive,
    /// Networking is active externally. This happens when the client is connected to a
    /// remote dedicated server.
    External,
    /// Networking is active internally. This happens when the client is running its own
    /// local server instance and connecting to it.
    Internal,
}
