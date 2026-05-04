use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum AppStartupPhase {
    Textures,
    Blocks,
    Biomes,
    RenderRegistry,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationLoadingPhase {
    FakeWork,
}
