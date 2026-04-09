// INFO: ----------------------
//         asset plugin
// ----------------------------

use bevy::app::{App, Plugin};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, _app: &mut App) {
        // register our custom asset types with Bevy's native asset system
    }
}
