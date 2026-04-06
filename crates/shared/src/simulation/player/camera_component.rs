use bevy::ecs::component::Component;
use bevy::math::{Mat4, Vec3};

/// A component that holds camera data for an entity
#[derive(Component)]
pub struct CameraComponent {
    // location
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub world_up: Vec3,

    // orientation
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,

    // matrices
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            front: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            right: Vec3::new(1.0, 0.0, 0.0),
            world_up: Vec3::new(0.0, 1.0, 0.0),

            yaw: -90.0,
            pitch: 0.0,
            zoom: 45.0,

            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
        }
    }
}
