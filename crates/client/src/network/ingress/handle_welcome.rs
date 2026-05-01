use crate::network::messages::WelcomeEvent;
use crate::player::components::LocalPlayer;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use shared::player::components::LogicalPosition;
use shared::world::chunk::ChunkCoord;

pub fn handle_welcome_system(
    mut ev_welcome: MessageReader<WelcomeEvent>,
    mut player_query: Query<
        (&mut LogicalPosition, &mut Transform, &mut ChunkCoord),
        With<LocalPlayer>,
    >,
) {
    for event in ev_welcome.read() {
        info!(
            "Received Welcome from server! Setting spawn position to {:?}",
            event.spawn_pos
        );

        if let Ok((mut pos, mut transform, mut coord)) = player_query.single_mut() {
            pos.0 = event.spawn_pos;
            transform.translation = event.spawn_pos;
            coord.pos = ChunkCoord::world_to_chunk_pos(event.spawn_pos);

            info!("Player position synced to server spawn.");
        } else {
            warn!("Welcome received but LocalPlayer not found!");
        }
    }
}
