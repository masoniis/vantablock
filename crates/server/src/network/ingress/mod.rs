use bevy::prelude::*;
use lightyear::prelude::*;
use shared::network::protocol::ClientMessage;

pub mod voxel_actions;

use self::voxel_actions::handle_client_voxel_requests;
use super::types::ClientConnection;

pub struct ServerIngressPlugin;

impl Plugin for ServerIngressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (receive_client_messages, handle_client_voxel_requests),
        );
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
                // update server-side look component from forward vector
                look.pitch = forward.y.asin();
                look.yaw = (-forward.z).atan2(forward.x) - std::f32::consts::FRAC_PI_2;
            }
        }
    }
}
