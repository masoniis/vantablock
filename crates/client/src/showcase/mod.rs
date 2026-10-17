pub mod systems;

use crate::input::local_actions::ClientAction;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use systems::{apply_default_showcase_system, apply_showcase_system};

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_default_showcase_system);

        app.add_systems(
            Update,
            apply_showcase_system.run_if(|query: Query<&ActionState<ClientAction>>| {
                let Some(action_state) = query.iter().next() else {
                    return false;
                };
                [
                    ClientAction::Showcase0,
                    ClientAction::Showcase1,
                    ClientAction::Showcase2,
                    ClientAction::Showcase3,
                    ClientAction::Showcase4,
                    ClientAction::Showcase5,
                    ClientAction::Showcase6,
                    ClientAction::Showcase7,
                    ClientAction::Showcase8,
                    ClientAction::Showcase9,
                ]
                .iter()
                .any(|a| action_state.just_pressed(a))
            }),
        );
    }
}
