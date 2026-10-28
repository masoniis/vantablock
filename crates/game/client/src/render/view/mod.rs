//! The camera as the render world sees it: the view uniform every pass binds, and the render
//! phases each camera gets.

pub mod camera_phases;
pub mod uniform;

pub use uniform::{
    CentralCameraViewBindGroupLayout, CentralCameraViewUniform, update_camera_view_buffer_system,
};
