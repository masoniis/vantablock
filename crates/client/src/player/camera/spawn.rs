use crate::{
    input::{get_default_input_map, get_default_local_input_map, local_actions::LocalClientAction},
    player::LocalPlayer,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use shared::{
    player::components::{LogicalPosition, NetworkPlayer, PlayerLook},
    player::PlayerAction,
    world::chunk::{ChunkCoord, CHUNK_SIDE_LENGTH},
};

const DEFAULT_CAMERA_STARTING_X: f32 = (CHUNK_SIDE_LENGTH / 2) as f32;
const DEFAULT_CAMERA_STARTING_Y: f32 = 64.0;
const DEFAULT_CAMERA_STARTING_Z: f32 = (CHUNK_SIDE_LENGTH / 2) as f32;

/// A system that spawns the initial player camera.
pub fn spawn_camera_system(mut commands: Commands) {
    info!("Spawning default graphics camera.");

    let start_position = Vec3::new(
        DEFAULT_CAMERA_STARTING_X,
        DEFAULT_CAMERA_STARTING_Y,
        DEFAULT_CAMERA_STARTING_Z,
    );

    let start_chunk = ChunkCoord::world_to_chunk_pos(start_position);

    // TODO: the client shouldnt spawn its own variant of player and stuff, it should be handled by replication
    // from the server instead to avoid dupes and stuff
    commands
        .spawn((
            NetworkPlayer,
            LocalPlayer,
            PlayerLook::default(),
            LogicalPosition(start_position),
            Transform::from_translation(start_position),
            ChunkCoord { pos: start_chunk },
            ActionState::<PlayerAction>::default(),
            get_default_input_map(),
            ActionState::<LocalClientAction>::default(),
            get_default_local_input_map(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3d::default(),
                Projection::Perspective(PerspectiveProjection {
                    fov: 45.0f32.to_radians(),
                    near: 1.0,
                    far: f32::INFINITY,
                    ..Default::default()
                }),
                Transform::from_xyz(0.0, 1.6, 0.0),
                ChunkCoord { pos: start_chunk },
            ));
        });
}
