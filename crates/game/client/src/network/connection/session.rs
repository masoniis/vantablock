use crate::{
    lifecycle::{ClientLifecycleState, InGameState},
    network::connection::NetworkErrorEvent,
};
use bevy::prelude::*;
use lightyear::prelude::{
    Client, Connected, Connecting, Disconnect, Disconnected, PredictionManager,
};

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
    mut next_client_state: ResMut<NextState<ClientLifecycleState>>,
    mut commands: Commands,
) {
    // did any entity stop connecting OR stop being connected this frame?
    for entity in removed_connecting.read().chain(removed_connected.read()) {
        // if the entity still exists and now has the Disconnected component,
        // a genuine network failure or drop occurred.
        if let Ok(disconnected) = disconnected_query.get(entity) {
            let reason_str = disconnected.reason.to_string();

            info!(
                "Client actively disconnected from server! Showing error screen. (Reason: {})",
                reason_str
            );

            next_client_state.set(ClientLifecycleState::Error);
            commands.trigger(NetworkErrorEvent {
                reason: reason_str.to_string(),
            });
        }
    }
}

/// Tears down the connection when leaving the game, whether to the main menu or the error
/// screen, so the next session starts from a single fresh client entity.
pub fn end_session(
    clients: Query<(Entity, Has<Disconnected>), With<Client>>,
    mut commands: Commands,
) {
    for (entity, disconnected) in clients.iter() {
        if !disconnected {
            commands.trigger(Disconnect { entity });
        }
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<PredictionManager>();
}
