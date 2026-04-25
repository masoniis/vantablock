use bevy::ecs::prelude::Message;

/// Triggered by the client to request a local singleplayer server.
/// Handled by the runner (orchestrator) to spin up a background server.
#[derive(Message, Debug, Clone)]
pub struct RequestSingleplayerSession;
