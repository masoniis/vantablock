use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

/// A unique identifier for a player.
///
/// This ID is assigned by the server and used to uniquely identify players across the network.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub u64);

/// Marker component for an entity that represents a player in the network.
///
/// **Authoritative Replication:** This component is managed and synchronized by the server.
/// Clients MUST NOT spawn entities with this component locally. They are spawned automatically
/// on the client via network replication when the server informs the client of a player's presence.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Default)]
pub struct NetworkPlayer;

/// The authoritative position of an entity within the fixed-step simulation.
///
/// This serves as the "source of truth" for physics and logic. On the client,
/// Bevy's native `Transform` is smoothly interpolated to chase this value
/// to provide high-framerate visual continuity.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Default)]
pub struct LogicalPosition(pub Vec3);

impl Ease for LogicalPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| Self(start.0.lerp(end.0, t)))
    }
}

/// Represents the orientation of a player's view.
///
/// Rotation is stored as Euler angles (yaw and pitch) in radians. This state is
/// used to calculate movement vectors and camera orientation.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
pub struct PlayerLook {
    /// Horizontal rotation (around the Y axis) in radians.
    pub yaw: f32,
    /// Vertical rotation (around the X axis) in radians.
    pub pitch: f32,
}

impl Default for PlayerLook {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

impl Ease for PlayerLook {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        // raw difference
        let delta_pitch = end.pitch - start.pitch;
        let mut delta_yaw = end.yaw - start.yaw;

        // wrap the yaw to find the shortest rotation path [-PI, PI]
        while delta_yaw > PI {
            delta_yaw -= TAU;
        }
        while delta_yaw < -PI {
            delta_yaw += TAU;
        }

        // final function curve
        FunctionCurve::new(Interval::EVERYWHERE, move |t| Self {
            yaw: start.yaw + delta_yaw * t,
            pitch: start.pitch + delta_pitch * t,
        })
    }
}
