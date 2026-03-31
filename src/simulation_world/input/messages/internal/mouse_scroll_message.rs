use bevy::math::Vec2;
use bevy_ecs::prelude::Message;

#[derive(Message, Debug, Clone)]
pub struct MouseScrollMessage {
    pub delta: Vec2,
}
