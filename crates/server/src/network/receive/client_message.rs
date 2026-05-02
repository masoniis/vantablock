use crate::network::types::ClientConnection;
use bevy::ecs::system::Query;
use lightyear::prelude::MessageReceiver;
use shared::{network::ClientMessage, player::components::PlayerLook};

/// Receives client messages and handles them
pub fn receive_client_messages(
    mut query: Query<(
        &mut MessageReceiver<ClientMessage>,
        &mut PlayerLook,
        &ClientConnection,
    )>,
) {
    for (mut receiver, mut look, _conn) in query.iter_mut() {
        for message in receiver.receive() {
            if let ClientMessage::UpdateView { forward } = message {
                // update server-side look component from forward vector
                look.pitch = forward.y.asin();
                look.yaw = (-forward.z).atan2(forward.x) - std::f32::consts::FRAC_PI_2;
            }
        }
    }
}
