use bevy::math::{DVec2, Vec2};
use bevy_ecs::prelude::Resource;

#[derive(Debug, Resource, Default)]
pub struct CursorMovement {
    mouse_delta: DVec2,
    scroll_delta: Vec2,
}

impl CursorMovement {
    pub fn adjust_mouse_delta(&mut self, delta: DVec2) {
        self.mouse_delta += delta;
    }

    pub fn adjust_scroll_delta(&mut self, delta: Vec2) {
        self.scroll_delta += delta;
    }

    pub fn reset_deltas(&mut self) {
        self.mouse_delta = DVec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }

    pub fn get_mouse_delta(&self) -> DVec2 {
        self.mouse_delta
    }

    pub fn get_scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }
}
