use bevy::prelude::*;
use lightyear::prelude::*;
use shared::network::protocol::ClientMessage;

use super::types::ClientConnection;

pub struct ServerIngressPlugin;

impl Plugin for ServerIngressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, receive_client_messages);
    }
}

pub fn receive_client_messages(
    mut query: Query<(
        &mut MessageReceiver<ClientMessage>,
        &mut shared::player::components::PlayerLook,
        &ClientConnection,
    )>,
) {
    for (mut receiver, mut look, _conn) in query.iter_mut() {
        for message in receiver.receive() {
            if let ClientMessage::UpdateView { forward } = message {
                // Update server-side look component based on forward vector
                // This is a simplified reconstruction of yaw/pitch
                look.pitch = forward.y.asin();
                look.yaw = (-forward.z).atan2(forward.x) - std::f32::consts::FRAC_PI_2;
            }
        }
    }
}
