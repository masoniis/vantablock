use crate::player::replicated_player::dress_local_player;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use shared::player::components::{LogicalPosition, NetworkPlayer, PlayerOwner};

/// A system that watches for new replicated players in the ECS world and dresses them with
/// the extra necessary components.
pub fn dress_replicated_players_system(
    mut commands: Commands,
    // query for newly replicated players that haven't been dressed yet.
    new_players: Query<(Entity, &PlayerOwner, &LogicalPosition), Added<NetworkPlayer>>,
    // get the local client's connection info to verify ownership
    netcode_client: Query<&NetcodeClient>,
) {
    let Some(client) = netcode_client.iter().next() else {
        return;
    };

    let local_client_id = client.id();

    for (entity, owner, logical_pos) in new_players.iter() {
        let spawn_pos = logical_pos.0;

        if owner.0 == local_client_id {
            info!("This is the LOCAL player! Dressing entity {:?}...", entity);
            dress_local_player(entity, spawn_pos, &mut commands);
        } else {
            info!("This is a REMOTE player! Dressing entity {:?}...", entity);
            // remote players still need a Transform for the smoothing/interpolation systems to work
            commands
                .entity(entity)
                .insert(Transform::from_translation(spawn_pos));
            // TODO: dress_remote_player(entity, &mut commands);
        }
    }
}
