use bevy::ecs::prelude::Resource;
use bevy::math::Vec2;

#[derive(Debug, Resource, Default)]
pub struct CursorMovement {
    /// mouse delta
    pub delta: Vec2,
    /// scroll delta
    pub scroll_delta: Vec2,
}

impl CursorMovement {
    pub fn reset_deltas(&mut self) {
        self.delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }

    pub fn adjust_mouse_delta(&mut self, delta: Vec2) {
        self.delta += delta;
    }

    pub fn adjust_scroll_delta(&mut self, delta: Vec2) {
        self.scroll_delta += delta;
    }
}
