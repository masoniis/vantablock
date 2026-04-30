// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::player::components::{LogicalPosition, PlayerLook};
use bevy::prelude::*;
use lightyear::prelude::{
    AppComponentExt, InterpolationRegistrationExt, PredictionRegistrationExt,
};

pub struct NetComponentsPlugin;

impl Plugin for NetComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_component::<PlayerLook>()
            .add_prediction()
            .add_linear_interpolation();

        app.register_component::<LogicalPosition>()
            .add_prediction()
            .add_linear_interpolation();
    }
}
