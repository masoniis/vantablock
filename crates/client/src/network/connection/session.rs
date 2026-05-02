use crate::lifecycle::{ClientState, InGameState};
use crate::network::connection::NetworkErrorEvent;
use bevy::prelude::*;
use lightyear::prelude::{Connected, Connecting, Disconnected};

pub fn handle_connections(
    trigger: On<Add, Connected>,
    mut next_in_game_state: ResMut<NextState<InGameState>>,
) {
    let server_entity = trigger.entity;

    // lightyear automatically adds message receiver and stuff on connection
    // so currently no need to do that here

    info!(
        "Client listening for messages from server! (entity {:?})",
        server_entity
    );

    // TODO: eventually will enter world loading state here, but for now
    // we just enter the raw world and wait for data as we play
    next_in_game_state.set(InGameState::Playing);
}

pub fn handle_disconnections(
    // listen for entities that lost their active states
    mut removed_connecting: RemovedComponents<Connecting>,
    mut removed_connected: RemovedComponents<Connected>,
    // query to see if they were given the Disconnected state
    disconnected_query: Query<&Disconnected>,
    mut next_client_state: ResMut<NextState<ClientState>>,
    mut commands: Commands,
) {
    // did any entity stop connecting OR stop being connected this frame?
    for entity in removed_connecting.read().chain(removed_connected.read()) {
        // if the entity still exists and now has the Disconnected component,
        // a genuine network failure or drop occurred.
        if let Ok(disconnected) = disconnected_query.get(entity) {
            let reason_str = disconnected
                .reason
                .as_deref()
                .unwrap_or("Graceful or Unknown");

            info!(
                "Client actively disconnected from server! Showing error screen. (Reason: {})",
                reason_str
            );

            next_client_state.set(ClientState::Error);
            commands.trigger(NetworkErrorEvent {
                reason: reason_str.to_string(),
            });
        }
    }
}
