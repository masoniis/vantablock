use shared::lifecycle::load::LoadingDagPhase;

/// Marker for the client launch loading phase.
///
/// This loading phase handles the transition from `ClientLifecycleState::Launching`
/// to `ClientLifecycleState::MainMenu`.
pub struct ClientLaunchLoadingPhase;

impl LoadingDagPhase for ClientLaunchLoadingPhase {
    const PHASE_NAME: &'static str = "ClientLaunchLoading";
}
