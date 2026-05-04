use bevy::prelude::*;
use shared::lifecycle::load::LoadingDagPhase;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum AppStartupPhase {
    Textures,
    Blocks,
    Biomes,
    RenderRegistry,
}

impl LoadingDagPhase for AppStartupPhase {
    const PHASE_NAME: &'static str = "AppStartupLoading";
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationLoadingPhase {
    FakeWork,
}

impl LoadingDagPhase for SimulationLoadingPhase {
    const PHASE_NAME: &'static str = "SimulationLoading";
}
