use bevy::app::{App, Plugin};

pub struct BlockRenderPlugin;

impl Plugin for BlockRenderPlugin {
    fn build(&self, _app: &mut App) {
        // block-specific render setup
    }
}
