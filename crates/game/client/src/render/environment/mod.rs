//! The sky and lighting inputs every pass shares: the sun (from the world clock), render time,
//! and the environment uniform built from them.

pub mod sun;
pub mod time;
pub mod uniform;

pub use sun::ExtractedSun;
pub use time::RenderTimeResource;
pub use uniform::{
    EnvironmentBindGroupLayout, EnvironmentUniforms, update_environment_uniform_buffer_system,
};
