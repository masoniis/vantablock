//! The shared world model: canonical assets and the server-owned objects placed from them.

pub mod assets;
pub mod objects;

pub use assets::{LoadVantaAssets, LoadedAsset, VantaAssetId, VantaAssetLibrary};
pub use objects::{ObjectAnimation, ObjectPlacement, WorldObject};

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::lifecycle::load::{AppStartupLoadingPhase, LoadingAppExt};
use bevy::app::{App, Plugin};

pub struct SharedWorldPlugin;

impl Plugin for SharedWorldPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WorldObject>()
            .register_type::<ObjectPlacement>()
            .register_type::<ObjectAnimation>();

        app.configure_loading_phase::<AppStartupLoadingPhase>()
            .add_node(LoadVantaAssets, assets::load_vanta_assets);
    }
}
