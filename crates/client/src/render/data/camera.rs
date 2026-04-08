use bevy::ecs::prelude::{Commands, Query, ResMut, Resource, With};
use bevy::prelude::{Camera, Camera3d, GlobalTransform, Mat4, Projection, Vec3};
use bevy::render::Extract;
use tracing::{instrument, warn};

/// A resource in the render world holding the extracted camera matrices.
#[derive(Resource, Debug)]
pub struct RenderCameraResource {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub world_position: Vec3,
}

/// A standalone system to extract the active camera's data from sim world.
#[instrument(skip_all)]
pub fn extract_active_camera_system(
    // input
    mut commands: Commands,
    camera_query: Extract<Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>>,

    // output (optional because it might not exist yet)
    mut render_camera: Option<ResMut<RenderCameraResource>>,
) {
    // find the active 3D camera
    let mut active_camera_data = None;
    for (camera, global_transform, projection) in camera_query.iter() {
        if camera.is_active {
            active_camera_data = Some((global_transform, projection));
            break;
        }
    }

    let Some((global_transform, projection)) = active_camera_data else {
        warn!("extract_active_camera_system: SimulationWorld has no active Camera3d.");
        return;
    };

    let projection_matrix = match projection {
        Projection::Perspective(p) => {
            Mat4::perspective_infinite_reverse_rh(p.fov, p.aspect_ratio, p.near)
        }
        _ => Mat4::IDENTITY,
    };

    let new_camera = RenderCameraResource {
        view_matrix: global_transform.to_matrix().inverse(),
        projection_matrix,
        world_position: global_transform.translation(),
    };

    // update the render world camera resource
    if let Some(ref mut target) = render_camera {
        **target = new_camera;
    } else {
        commands.insert_resource(new_camera);
    }
}
