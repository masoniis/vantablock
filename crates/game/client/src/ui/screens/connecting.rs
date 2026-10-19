use crate::{
    lifecycle::state::InGameState,
    ui::{screens::status_card, theme::backdrop},
};
use bevy::prelude::*;

#[derive(Component, Default, Clone)]
pub struct ConnectingUiRoot;

pub struct ConnectingUiPlugin;

impl Plugin for ConnectingUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(InGameState::Connecting), spawn_connecting_ui);
    }
}

fn spawn_connecting_ui(mut commands: Commands) {
    info!("Spawning Connecting UI...");
    commands
        .spawn_scene(bsn! {
            ConnectingUiRoot
            backdrop()
            Children [ status_card("Connecting", "Establishing connection") ]
        })
        .insert(DespawnOnExit(InGameState::Connecting));
}
