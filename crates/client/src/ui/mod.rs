use bevy::prelude::*;
use shared::lifecycle::state::SimulationState;
use shared::lifecycle::state::enums::AppState;

pub mod systems;

// INFO: -------------------
//         ui plugin
// -------------------------

pub struct VantablockUiPlugin;

impl Plugin for VantablockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::Running),
            systems::spawning::spawn_ui_system,
        )
        .add_systems(
            OnExit(AppState::Running),
            systems::spawning::despawn_ui_system,
        );

        // starting up ui
        app.add_systems(
            OnEnter(SimulationState::Loading),
            systems::starting_up_ui::spawn_starting_up_ui,
        )
        .add_systems(
            OnExit(SimulationState::Loading),
            systems::starting_up_ui::despawn_starting_up_ui,
        );
    }
}
