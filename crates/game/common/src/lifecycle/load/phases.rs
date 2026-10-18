use crate::lifecycle::load::LoadingDagPhase;

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
