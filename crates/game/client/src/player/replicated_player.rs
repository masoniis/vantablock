use crate::{
    input::input_maps::get_default_player_action_input_map,
    player::components::{LocalPlayer, LocalPlayerCamera, LocalPlayerLook},
    render::VoxelCamera,
};
use bevy::{prelude::*, render::view::Msaa};
use common::player::PlayerAction;
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
            Transform::from_translation(spawn_pos),
            // the camera child inherits visibility from here
            Visibility::default(),
        ))
        .with_children(|parent| {
            // TODO: need a proper transition from dressing the local player to actually rendering the world,
            // that way we can seamless delete the 2D main menu camera and enable the 3d render camera with no
            // gap
            parent.spawn((
                Camera3d::default(),
                LocalPlayerCamera,
                // the voxel renderer traces every pixel itself, so the view targets stay
                // single-sampled
                VoxelCamera,
                Msaa::Off,
                Projection::Perspective(PerspectiveProjection {
                    fov: 60.0f32.to_radians(),
                    near: 0.05,
                    far: f32::INFINITY,
                    ..Default::default()
                }),
            ));
        });
}
