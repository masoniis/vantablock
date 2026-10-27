// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use bevy::app::{App, Plugin};

pub struct NetworkSendPlugin;

impl Plugin for NetworkSendPlugin {
    fn build(&self, _app: &mut App) {}
}
