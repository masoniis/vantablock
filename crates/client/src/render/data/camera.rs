use crate::prelude::*;
use bevy::ecs::prelude::{Commands, Query, Res, ResMut, Resource};
use bevy::render::Extract;
use shared::simulation::player::active_camera::ActiveCamera;
use shared::simulation::player::camera_component::CameraComponent;

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
    // Input
    mut commands: Commands,
    active_camera_res: Extract<Option<Res<ActiveCamera>>>,
    camera_query: Extract<Query<&CameraComponent>>,

    // Output (optional because it might not exist yet)
    render_camera: Option<ResMut<RenderCameraResource>>,
) {
    // get the ActiveCamera resource from the simulation world
    let active_camera_res = match active_camera_res.as_ref() {
        Some(res) => res,
        None => {
            warn!("extract_active_camera_system: SimulationWorld has no ActiveCamera resource.");
            return;
        }
    };
    let active_entity = active_camera_res.0;

    // get the CameraComponent for that entity
    let source_component = match camera_query.get(active_entity) {
        Ok(comp) => comp,
        Err(_) => {
            warn!(
                "extract_active_camera_system: ActiveCamera entity {:?} has no CameraComponent.",
                active_entity
            );
            return; // entity exists but component is missing
        }
    };

    let new_camera = RenderCameraResource {
        view_matrix: source_component.view_matrix,
        projection_matrix: source_component.projection_matrix,
        world_position: source_component.position,
    };

    // update the render world camera resource
    if let Some(mut target) = render_camera {
        *target = new_camera;
    } else {
        commands.insert_resource(new_camera);
    }
}
