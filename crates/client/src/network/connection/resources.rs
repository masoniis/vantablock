use bevy::prelude::*;

#[derive(Event, Debug, Clone)]
pub struct NetworkErrorEvent {
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectType {
    Singleplayer,
    Multiplayer,
}

/// An event to initiate a connection to a specific server address.
#[derive(Event, Debug)]
pub struct InitiateConnection {
    pub connect_type: ConnectType,
    pub server_addr: String,
}

/// Triggered by the client to request a local singleplayer server.
/// Handled by the runner (orchestrator) to spin up a background server.
#[derive(Event, Debug, Clone)]
pub struct RequestSingleplayerSession;
