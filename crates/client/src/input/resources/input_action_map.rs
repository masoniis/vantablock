use bevy::ecs::prelude::Resource;
use bevy::prelude::{KeyCode, MouseButton};
use shared::simulation::input::types::SimulationAction;
use std::collections::hash_map::{HashMap, Iter};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Input {
    Key(KeyCode),
    MouseButton(MouseButton),
}

/// A map from input keys to an action. Set as a resource
/// which means it can be configured by systems at runtime.
#[derive(Debug, Resource)]
pub struct InputActionMapResource {
    bindings: HashMap<Input, SimulationAction>,
}

impl InputActionMapResource {
    /// Gets the game action associated with a given input, if one exists.
    pub fn get_action(&self, input: &Input) -> Option<&SimulationAction> {
        self.bindings.get(input)
    }

    /// Provides an iterator over all the currently configured input bindings.
    pub fn iter<'a>(&'a self) -> Iter<'a, Input, SimulationAction> {
        self.bindings.iter()
    }
}

impl Default for InputActionMapResource {
    fn default() -> Self {
        Self {
            bindings: HashMap::from([
                // Core player movement
                (Input::Key(KeyCode::KeyW), SimulationAction::MoveForward),
                (Input::Key(KeyCode::KeyS), SimulationAction::MoveBackward),
                (Input::Key(KeyCode::KeyA), SimulationAction::MoveLeft),
                (Input::Key(KeyCode::KeyD), SimulationAction::MoveRight),
                (Input::Key(KeyCode::ShiftLeft), SimulationAction::MoveFaster),
                // Core player actions
                (
                    Input::MouseButton(MouseButton::Left),
                    SimulationAction::BreakVoxel,
                ),
                (
                    Input::MouseButton(MouseButton::Right),
                    SimulationAction::PlaceVoxel,
                ),
                // Terrain gen
                (
                    Input::Key(KeyCode::KeyT),
                    SimulationAction::CycleActiveTerrainGenerator,
                ),
                // Game time control
                (
                    Input::Key(KeyCode::ArrowRight),
                    SimulationAction::JumpGameTimeForward,
                ),
                (
                    Input::Key(KeyCode::ArrowLeft),
                    SimulationAction::JumpGameTimeBackward,
                ),
                (Input::Key(KeyCode::Space), SimulationAction::PauseGameTime),
                // Misc
                (Input::Key(KeyCode::Escape), SimulationAction::TogglePause),
                // Debug/analysis tools
                (Input::Key(KeyCode::F1), SimulationAction::ToggleDiagnostics),
                (
                    Input::Key(KeyCode::KeyU),
                    SimulationAction::ToggleDiagnostics,
                ),
                (
                    Input::Key(KeyCode::F2),
                    SimulationAction::ToggleOpaqueWireframeMode,
                ),
                (
                    Input::Key(KeyCode::KeyO),
                    SimulationAction::ToggleOpaqueWireframeMode,
                ),
                (
                    Input::Key(KeyCode::F3),
                    SimulationAction::ToggleChunkBorders,
                ),
                (
                    Input::Key(KeyCode::KeyB),
                    SimulationAction::ToggleChunkBorders,
                ),
                // Showcase actions
                (Input::Key(KeyCode::Digit0), SimulationAction::Showcase0),
                (Input::Key(KeyCode::Digit1), SimulationAction::Showcase1),
                (Input::Key(KeyCode::Digit2), SimulationAction::Showcase2),
                (Input::Key(KeyCode::Digit3), SimulationAction::Showcase3),
                (Input::Key(KeyCode::Digit4), SimulationAction::Showcase4),
                (Input::Key(KeyCode::Digit5), SimulationAction::Showcase5),
                (Input::Key(KeyCode::Digit6), SimulationAction::Showcase6),
                (Input::Key(KeyCode::Digit7), SimulationAction::Showcase7),
                (Input::Key(KeyCode::Digit8), SimulationAction::Showcase8),
                (Input::Key(KeyCode::Digit9), SimulationAction::Showcase9),
            ]),
        }
    }
}
