mod wireframe_vertex;

pub use crate::render::passes::VoxelMesh;
pub use shared::simulation::chunk::meshing::packed_face::PackedFace;
pub use wireframe_vertex::WireframeVertex;

use bevy::ecs::component::Component;
use bevy::math::Mat4;

/// A component representing a transform on a mesh in the render world.
#[derive(Component, Clone)]
pub struct RenderTransformComponent {
    pub transform: Mat4,
}
