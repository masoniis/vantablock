use bevy::ecs::prelude::Message;
use bevy::math::Vec2;

#[derive(Message, Debug, Clone)]
pub struct MouseScrollMessage {
    pub delta: Vec2,
}
