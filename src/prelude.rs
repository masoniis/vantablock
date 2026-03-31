/// Prelude is to be reserved for imports that get used across many
/// files. For this project, that mostly includes things that are used
/// at a system ecs module level (eg plugin) as there will be many modules.
// INFO: -----------------------
//         crate imports
// -----------------------------
pub use crate::{
    ecs_core::{
        state_machine::{in_state, AppState, GameState},
        CommonEcsInterface, EcsBuilder, Plugin, PluginGroup, ScheduleBuilder,
    },
    render_world::scheduling::*,
    simulation_world::{input::SimulationAction, scheduling::*},
    utils::*,
};

// INFO: ----------------------
//         useful utils
// ----------------------------

pub use bevy::math::{
    vec2, vec3, vec4, FloatExt, IVec2, IVec3, IVec4, Mat3, Mat4, Quat, Vec2, Vec3, Vec4,
    Vec4Swizzles,
};
pub use derive_more::{Deref, DerefMut};
pub use std::f32::consts::{FRAC_PI_2, PI};
pub use std::sync::Arc;
pub use tracing::{debug, error, info, info_span, instrument, trace, warn};
pub use winit::{
    dpi::{LogicalSize, PhysicalSize},
    keyboard::KeyCode,
};
