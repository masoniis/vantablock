use bevy::prelude::*;
use shared::lifecycle::load::LoadingDagPhase;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationLoadingPhase {
    Biomes,
    Blocks,
}

impl LoadingDagPhase for SimulationLoadingPhase {
    const PHASE_NAME: &'static str = "SimulationLoading";
}
