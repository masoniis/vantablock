use bevy::prelude::{Resource, States};

#[derive(States, Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServerAppState {
    #[default]
    Initializing, // Loading server config, binding ports
    GeneratingSpawn, // Generating initial chunks before allowing players
    Running,         // Accepting connections, ticking simulation
    ShuttingDown,    // Saving chunks, kicking players gracefully
}
