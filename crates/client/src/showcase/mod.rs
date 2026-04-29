pub mod systems;

use crate::input::local_actions::LocalClientAction;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use systems::{apply_default_showcase_system, apply_showcase_system};

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_default_showcase_system);

        app.add_systems(
            Update,
            apply_showcase_system.run_if(|query: Query<&ActionState<LocalClientAction>>| {
                let Some(action_state) = query.iter().next() else {
                    return false;
                };
                [
                    LocalClientAction::Showcase0,
                    LocalClientAction::Showcase1,
                    LocalClientAction::Showcase2,
                    LocalClientAction::Showcase3,
                    LocalClientAction::Showcase4,
                    LocalClientAction::Showcase5,
                    LocalClientAction::Showcase6,
                    LocalClientAction::Showcase7,
                    LocalClientAction::Showcase8,
                    LocalClientAction::Showcase9,
                ]
                .iter()
                .any(|a| action_state.just_pressed(a))
            }),
        );
    }
}
