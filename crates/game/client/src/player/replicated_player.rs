use crate::{
    input::{
        input_maps::{get_default_client_action_input_map, get_default_player_action_input_map},
        local_actions::ClientAction,
    },
    player::components::{LocalPlayer, LocalPlayerCamera, LocalPlayerLook},
};
use bevy::prelude::*;
use common::{player::PlayerAction, world::chunk::ChunkCoord};
use leafwing_input_manager::prelude::ActionState;

/// Attaches all local-only components and children to the local player entity.
///
/// `PlayerLook` and `LogicalPosition` are deliberately not inserted here: they are predicted
/// components that lightyear already placed on the entity, and overwriting them would
/// desync the prediction history.
pub fn dress_local_player(entity: Entity, spawn_pos: Vec3, commands: &mut Commands) {
    commands
        .entity(entity)
        .insert((
            LocalPlayer,
            LocalPlayerLook::default(),
            ActionState::<PlayerAction>::default(),
            get_default_player_action_input_map(),
            ActionState::<ClientAction>::default(),
            get_default_client_action_input_map(),
            Transform::from_translation(spawn_pos),
            ChunkCoord {
                pos: ChunkCoord::world_to_chunk_pos(spawn_pos),
            },
        ))
        .with_children(|parent| {
            // TODO: need a proper transition from dressing the local player to actually rendering the world,
            // that way we can seamless delete the 2D main menu camera and enable the 3d render camera with no
            // gap
            parent.spawn((
                Camera3d::default(),
                LocalPlayerCamera,
                Projection::Perspective(PerspectiveProjection {
                    fov: 45.0f32.to_radians(),
                    near: 0.1,
                    far: f32::INFINITY,
                    ..Default::default()
                }),
                Transform::from_xyz(0.0, 1.6, 0.0),
                ChunkCoord {
                    pos: ChunkCoord::world_to_chunk_pos(spawn_pos + Vec3::new(0.0, 1.6, 0.0)),
                },
            ));
        });
}
