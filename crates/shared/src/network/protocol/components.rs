// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{
    player::components::{LogicalPosition, NetworkPlayer, PlayerLook, PlayerOwner},
    world::objects::{ObjectAnimation, ObjectPlacement, WorldObject},
};
use bevy::prelude::*;
use lightyear::prelude::*;

pub struct NetComponentsPlugin;

impl Plugin for NetComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_component::<NetworkPlayer>();
        app.register_component::<PlayerOwner>();

        app.register_component::<PlayerLook>()
            .add_prediction()
            .add_linear_interpolation();

        app.register_component::<LogicalPosition>()
            .add_prediction()
            .add_linear_interpolation();

        // world objects are server-owned and only ever replicated as-is
        app.register_component::<WorldObject>();
        app.register_component::<ObjectPlacement>();
        app.register_component::<ObjectAnimation>();
    }
}
