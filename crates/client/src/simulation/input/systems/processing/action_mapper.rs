use bevy::ecs::system::{Res, ResMut};
use bevy::prelude::{ButtonInput, KeyCode, MouseButton};
use shared::simulation::input::resources::{
    InputActionMapResource, action::ActionStateResource, input_action_map::Input,
};
use tracing::instrument;

/// A system that translates the raw state from Bevy's native input resources into abstract,
/// game-specific actions based on the current `InputActionMapResource`.
#[instrument(skip_all)]
pub fn update_action_state_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    action_map: Res<InputActionMapResource>,
    mut action_state: ResMut<ActionStateResource>,
) {
    // INFO: -----------------------------------------
    //         Update the actions
    // -----------------------------------------------

    // We clear the state of "one-shot" actions (just pressed/just released)
    // before we update them again.
    action_state.clear();

    // We iterate through every bound action and update its state based on whether
    // the associated key is pressed, was just pressed, or was just released.
    for (input, action) in action_map.iter() {
        let is_down = match input {
            Input::Key(code) => keyboard_input.pressed(*code),
            Input::MouseButton(button) => mouse_input.pressed(*button),
        };

        let just_pressed = match input {
            Input::Key(code) => keyboard_input.just_pressed(*code),
            Input::MouseButton(button) => mouse_input.just_pressed(*button),
        };

        let just_released = match input {
            Input::Key(code) => keyboard_input.just_released(*code),
            Input::MouseButton(button) => mouse_input.just_released(*button),
        };

        // update state
        if is_down {
            action_state.hold(*action);
        }

        if just_pressed {
            action_state.press(*action);
        }

        if just_released {
            action_state.release(*action);
        }
    }
}
