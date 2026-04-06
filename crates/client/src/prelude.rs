/// Prelude is to be reserved for imports that get used across many
/// files. For this project, that mostly includes things that are used
/// at a system ecs module level (eg plugin) as there will be many modules.
// INFO: -----------------------
//         crate imports
// -----------------------------
pub use crate::render::scheduling::*;
pub use utils::*;

// INFO: ----------------------
//         useful utils
// ----------------------------

pub use bevy::math::{
    FloatExt, IVec2, IVec3, IVec4, Mat3, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles, vec2, vec3,
    vec4,
};
pub use derive_more::{Deref, DerefMut};
pub use std::f32::consts::{FRAC_PI_2, PI};
pub use std::sync::Arc;
pub use tracing::{debug, error, info, info_span, instrument, trace, warn};
