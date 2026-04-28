pub mod systems;

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use shared::simulation::player::PlayerAction;
use systems::{apply_default_showcase_system, apply_showcase_system};

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_default_showcase_system);

        app.add_systems(
            Update,
            apply_showcase_system.run_if(|query: Query<&ActionState<PlayerAction>>| {
                let Some(action_state) = query.iter().next() else {
                    return false;
                };
                [
                    PlayerAction::Showcase0,
                    PlayerAction::Showcase1,
                    PlayerAction::Showcase2,
                    PlayerAction::Showcase3,
                    PlayerAction::Showcase4,
                    PlayerAction::Showcase5,
                    PlayerAction::Showcase6,
                    PlayerAction::Showcase7,
                    PlayerAction::Showcase8,
                    PlayerAction::Showcase9,
                ]
                .iter()
                .any(|a| action_state.just_pressed(a))
            }),
        );
    }
}
