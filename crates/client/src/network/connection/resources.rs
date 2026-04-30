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
    pub server_addr: String,
}

#[derive(Resource, Debug, Clone)]
pub struct ConnectionSettings {
    pub connect_type: ConnectType,
    pub server_addr: String,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            connect_type: ConnectType::Singleplayer,
            server_addr: "127.0.0.1:5000".to_string(),
        }
    }
}
