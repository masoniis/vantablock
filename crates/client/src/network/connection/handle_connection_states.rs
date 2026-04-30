use crate::lifecycle::ClientState;
use bevy::prelude::*;
use lightyear::prelude::{Connected, Connecting, Disconnected, MessageReceiver};
use shared::network::protocol::server::ServerMessage;

pub fn handle_connections(trigger: On<Add, Connected>, mut commands: Commands) {
    let server_entity = trigger.entity;

    // ensure server entity has MessageReceiver
    commands
        .entity(server_entity)
        .insert(MessageReceiver::<ServerMessage>::default());

    info!(
        "Client listening for messages from server! (entity {:?})",
        server_entity
    );
}

pub fn handle_disconnections(
    // listen for entities that lost their active states
    mut removed_connecting: RemovedComponents<Connecting>,
    mut removed_connected: RemovedComponents<Connected>,
    // query to see if they were given the Disconnected state
    disconnected_query: Query<&Disconnected>,
    mut next_client_state: ResMut<NextState<ClientState>>,
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
                "Client actively disconnected from server! Returning to main menu. (Reason: {})",
                reason_str
            );

            next_client_state.set(ClientState::MainMenu);
        }
    }
}
