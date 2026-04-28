use bevy::prelude::*;

pub mod components;
pub mod datagen;
pub mod manager;

pub use manager::{ServerChunkManager, ServerChunkState};

pub struct ServerChunkPlugin;

impl Plugin for ServerChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ServerChunkManager>();
    }
}
