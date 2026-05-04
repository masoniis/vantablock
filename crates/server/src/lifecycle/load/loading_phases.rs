use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationLoadingPhase {
    Biomes,
    Blocks,
}
