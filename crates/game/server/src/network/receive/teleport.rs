use crate::network::{receive::ecs_messages::InboundTeleportMessage, types::ClientConnection};
use bevy::prelude::*;
use common::player::components::LogicalPosition;

/// Moves each requesting client's player to the position it asked for.
///
/// The position is predicted, so the client corrects to it through a rollback.
pub fn handle_teleport_requests(
    mut requests: MessageReader<InboundTeleportMessage>,
    mut players: Query<(&ClientConnection, &mut LogicalPosition)>,
) {
    for request in requests.read() {
        let Some((_, mut position)) = players
            .iter_mut()
            .find(|(connection, _)| connection.client_entity == request.player)
        else {
            warn!(
                "Teleport requested by {:?}, which has no player",
                request.player
            );
            continue;
        };
        info!(
            "Teleporting player of {:?} to {}",
            request.player, request.position
        );
        position.0 = request.position;
    }
}
