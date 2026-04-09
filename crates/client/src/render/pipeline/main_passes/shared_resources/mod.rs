pub mod environment_uniform;
pub mod tex_array_uniforms;
pub mod view_uniforms;

use bevy::render::render_resource::TextureFormat;

pub const MAIN_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub use environment_uniform::{
    EnvironmentBindGroupLayout, EnvironmentUniforms, update_environment_uniform_buffer_system,
};
pub use tex_array_uniforms::{
    TextureArrayBindGroupLayout, TextureArrayUniforms, prepare_texture_array_system,
};
pub use view_uniforms::{
    CentralCameraViewBindGroupLayout, CentralCameraViewUniform, update_camera_view_buffer_system,
};
