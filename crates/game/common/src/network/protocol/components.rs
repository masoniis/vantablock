// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::player::components::{LogicalPosition, NetworkPlayer, PlayerLook, PlayerOwner};
use bevy::prelude::*;
use lightyear::prelude::*;

pub struct NetComponentsPlugin;

impl Plugin for NetComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.component::<NetworkPlayer>().replicate();
        app.component::<PlayerOwner>().replicate();

        app.component::<PlayerLook>()
            .replicate()
            .predict()
            .add_linear_interpolation();

        app.component::<LogicalPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
    }
}
