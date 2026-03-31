use bevy::math::DVec2;
use bevy_ecs::prelude::Message;

#[derive(Message, Debug, Clone)]
pub struct MouseMoveMessage {
    pub delta: DVec2,
}
