use bevy::ecs::component::Component;
use bevy::math::Mat4;

/// A component representing a transform on a mesh in the render world.
// TODO: bevy likely has a builtin
#[derive(Component, Clone)]
pub struct RenderTransformComponent {
    pub transform: Mat4,
}
