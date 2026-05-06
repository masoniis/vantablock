use crate::lifecycle::load::LoadingDagPhase;
use bevy::prelude::*;

/// Marker for the app startup loading phase.
///
/// This loading phase exists to handle the transition from the
/// `AppState::StartingUp` state to the `AppState::Running` state.
///
/// Anything loaded during this phase will stall the app from running
/// until loading is complete. This means is is crucial to ensure that
/// the only tasks assigned here are necessary startup tasks!
pub struct AppStartupLoadingPhase;

impl LoadingDagPhase for AppStartupLoadingPhase {
    const PHASE_NAME: &'static str = "AppStartupLoading";
}

/// Marker node for loading block definitions.
#[derive(Component)]
pub struct LoadBlocks;

/// Marker node for loading biome definitions.
#[derive(Component)]
pub struct LoadBiomes;
