use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectType {
    Singleplayer,
    Multiplayer,
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
