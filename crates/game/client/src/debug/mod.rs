//! Tools for looking inside the engine rather than playing the game: debug views (chunk
//! borders, opaque wireframe), the showcase scenes, and the F3 overlay.
//!
//! `DebugToolsPlugin` is part of the core plugins, so headless clients have the toggles and the
//! showcase too. The overlay draws UI, so the UI plugin adds `overlay::DebugOverlayPlugin`.

pub mod chunk_borders;
pub mod overlay;
pub mod showcase;
pub mod wireframe;

// INFO: ---------------------------
//         plugin definition
// ---------------------------------

use crate::{input::local_actions::ClientAction, lifecycle::ClientLifecycleState};
use bevy::{prelude::*, render::extract_resource::ExtractResourcePlugin};
use chunk_borders::{ChunkBoundsToggle, toggle_chunk_borders_system};
use leafwing_input_manager::common_conditions::action_just_pressed;
use wireframe::{OpaqueRenderMode, toggle_opaque_wireframe_mode_system};

pub struct DebugToolsPlugin;

impl Plugin for DebugToolsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OpaqueRenderMode>();
        app.init_resource::<ChunkBoundsToggle>();
        app.add_plugins(ExtractResourcePlugin::<ChunkBoundsToggle>::default());
        app.add_plugins(showcase::ShowcasePlugin);

        app.add_systems(
            Update,
            (
                toggle_opaque_wireframe_mode_system
                    .run_if(action_just_pressed(ClientAction::ToggleOpaqueWireframeMode)),
                toggle_chunk_borders_system
                    .run_if(action_just_pressed(ClientAction::ToggleChunkBorders)),
            )
                .run_if(in_state(ClientLifecycleState::InGame)),
        );
    }
}
