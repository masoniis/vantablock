use bevy::prelude::*;
use common::world::terrain::WorldGenSettings;

#[derive(Event, Debug, Clone)]
pub struct NetworkErrorEvent {
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectType {
    /// Start a local server generating a new world with these settings, and join it.
    Singleplayer(WorldGenSettings),
    Multiplayer,
}

/// An event to initiate a connection to a specific server address.
#[derive(Event, Debug)]
pub struct InitiateConnection {
    pub connect_type: ConnectType,
    pub server_addr: String,
}

/// Triggered by the client to request a local singleplayer server generating a new world.
/// Handled by the runner (orchestrator) to spin up a background server.
#[derive(Event, Debug, Clone)]
pub struct RequestSingleplayerSession {
    pub world: WorldGenSettings,
}

/// The settings of the world this session plays in, when the client knows them (it created
/// the world). Absent when joining a remote server.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SessionWorld(pub WorldGenSettings);
