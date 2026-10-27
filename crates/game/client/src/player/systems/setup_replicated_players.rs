use crate::player::replicated_player::dress_local_player;
use bevy::prelude::*;
use common::player::components::{LogicalPosition, NetworkPlayer};
use lightyear::prelude::*;

/// A system that watches for new replicated players in the ECS world and dresses them with
/// the extra necessary components.
///
/// The server marks the owning client's player as predicted and every other player as
/// interpolated, so the `Predicted` marker is what identifies the local player.
pub fn dress_replicated_players_system(
    mut commands: Commands,
    // query for newly replicated players that haven't been dressed yet.
    new_players: Query<
        (
            Entity,
            Has<Predicted>,
            Has<Interpolated>,
            Option<&Confirmed<LogicalPosition>>,
        ),
        Added<NetworkPlayer>,
    >,
) {
    for (entity, predicted, interpolated, confirmed_pos) in new_players.iter() {
        let spawn_pos = confirmed_pos.map(|pos| pos.0.0).unwrap_or_default();

        if predicted {
            info!("This is the LOCAL player! Dressing entity {:?}...", entity);
            dress_local_player(entity, spawn_pos, &mut commands);
        } else if interpolated {
            info!("This is a REMOTE player! Dressing entity {:?}...", entity);
            // remote players still need a Transform for the smoothing/interpolation systems to work
            commands
                .entity(entity)
                .insert(Transform::from_translation(spawn_pos));
            // TODO: dress_remote_player(entity, &mut commands);
        } else {
            warn!(
                "Replicated player {:?} is neither predicted nor interpolated, ignoring",
                entity
            );
        }
    }
}
