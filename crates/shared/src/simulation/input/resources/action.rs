use crate::simulation::input::types::simulation_action::SimulationAction;
use bevy::ecs::prelude::Resource;
use std::collections::HashSet;

/// A resource for central mapping of input to actions
///
/// This abstracts away *why* an action occurred and allows systems
/// to react to the event itself (as opposed to reacting to a key).
#[derive(Debug, Resource, Default)]
pub struct ActionStateResource {
    /// Actions that just happened in this frame.
    just_happened: HashSet<SimulationAction>,

    /// Actions that happened in a previous frame but are still ongoing.
    ongoing: HashSet<SimulationAction>,

    /// Actions that ended or were released this frame.
    ended: HashSet<SimulationAction>,
}

impl ActionStateResource {
    // INFO: ---------------------------
    //        State manipulation
    // ---------------------------------

    /// Clears the `just_happened` and `ended` sets.
    pub fn clear(&mut self) {
        self.just_happened.clear();
        self.ended.clear();
    }

    /// Marks an action as pressed.
    pub fn press(&mut self, action: SimulationAction) {
        self.just_happened.insert(action);
    }

    /// Marks an action as held.
    pub fn hold(&mut self, action: SimulationAction) {
        self.ongoing.insert(action);
    }

    /// Marks an action as released.
    pub fn release(&mut self, action: SimulationAction) {
        if self.ongoing.remove(&action) {
            self.ended.insert(action);
        }
    }

    // INFO: -----------------------
    //        State checking
    // -----------------------------

    /// Was the key associated with this action just pressed this frame?
    pub fn just_happened(&self, action: SimulationAction) -> bool {
        self.just_happened.contains(&action)
    }

    /// Is the key associated with this action being held?
    pub fn is_ongoing(&self, action: SimulationAction) -> bool {
        self.ongoing.contains(&action)
    }

    /// Was the key associated with this action just released?
    pub fn just_ended(&self, action: SimulationAction) -> bool {
        self.ended.contains(&action)
    }
}
