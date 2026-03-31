use bevy::ecs::prelude::Message;
use bevy::math::DVec2;

#[derive(Message, Debug, Clone)]
pub struct MouseMoveMessage {
    pub delta: DVec2,
}
