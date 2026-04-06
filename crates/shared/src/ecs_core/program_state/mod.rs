// The state machine module provides a flexible way to manage any transitionary state.
//
// This is important for both ECS worlds and thus it is in ecs_core. It provides utilities
// for run conditions based on the current state, for example, and is very generic overall.

pub mod app_state;
pub mod game_state;

pub use app_state::AppState;
pub use game_state::GameState;
