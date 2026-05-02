use bevy::prelude::States;

/// The current state of the highest level application lifecycle.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    /// The app is loading up essential data. The run loop has not started.
    ///
    /// The only work done during this phase should be work required for the window
    /// to render a startup/loading screen. Any non-essential work done during this
    /// state is just extra time spent hiding the main window/gui.
    StartingUp,
    /// The main application loop is active and executing.
    ///
    /// This is where the entire app logic is executed.
    Running,
    /// The application is performing cleanup operations before process termination.
    ///
    /// This is the final state of the app. Any essential saving or final actions must
    /// be done during this state, as it is their last chance.
    ShuttingDown,
}
