//! Preset scenes that show off the engine: a place, a view direction, and a time of day, in a
//! world generated with a particular terrain shaper. Picked from the pause menu's showcase
//! page, which triggers [`ApplyShowcase`].

use crate::{
    network::{OutboundSetTimeOfDayMessage, OutboundTeleportMessage},
    player::{LocalPlayer, LocalPlayerLook},
};
use bevy::prelude::*;
use common::world::terrain::TerrainShaperKind;

pub struct Showcase {
    pub name: &'static str,
    /// The terrain the scene was built for; it only makes sense in a world generated with it.
    pub shaper: TerrainShaperKind,
    /// Fraction of the day, from 0.0 (midnight) to 1.0.
    pub time_of_day: f32,
    pub position: Vec3,
    /// Degrees, in the old camera convention (see [`apply_showcase`]).
    pub yaw: f32,
    /// Degrees, positive looks up.
    pub pitch: f32,
}

pub const SHOWCASES: &[Showcase] = &[
    Showcase {
        name: "Flat introduction",
        shaper: TerrainShaperKind::Superflat,
        time_of_day: 0.25, // sun shining from the left
        position: Vec3::new(0.0, 68.5, 0.0),
        yaw: 85.0,
        pitch: -12.0,
    },
    Showcase {
        name: "Sine waves",
        shaper: TerrainShaperKind::Sinwave,
        time_of_day: 0.3,
        position: Vec3::new(-661.0, 68.5, 175.0),
        yaw: 85.0,
        pitch: -12.0,
    },
    Showcase {
        name: "Sunset horizon",
        shaper: TerrainShaperKind::Realistic,
        time_of_day: 0.73,
        position: Vec3::new(939.0, 71.2, 1218.0),
        yaw: -169.0,
        pitch: -3.0,
    },
    Showcase {
        name: "3D terrain",
        shaper: TerrainShaperKind::Simplex3d,
        time_of_day: 0.25,
        position: Vec3::new(1425.0, 73.0, 1984.0),
        yaw: 48.0,
        pitch: 7.0,
    },
    Showcase {
        name: "Bumps",
        shaper: TerrainShaperKind::Realistic,
        time_of_day: 0.223,
        position: Vec3::new(3696.0, 105.0, 474.0),
        yaw: 0.0,
        pitch: -3.0,
    },
    Showcase {
        name: "Beach",
        shaper: TerrainShaperKind::Realistic,
        time_of_day: 0.25,
        position: Vec3::new(37239.0, 75.1, -2423.0),
        yaw: 30.0,
        pitch: 6.0,
    },
    Showcase {
        name: "Badlands",
        shaper: TerrainShaperKind::Realistic,
        time_of_day: 0.25,
        position: Vec3::new(38250.0, 136.5, 416.6),
        yaw: -197.0,
        pitch: -33.0,
    },
];

/// Moves the local player to the showcase at this index in [`SHOWCASES`].
#[derive(Event)]
pub struct ApplyShowcase(pub usize);

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(apply_showcase);
    }
}

/// Asks the server to move the player and set the time of day (both are authoritative on the
/// server), and sets the look directly.
fn apply_showcase(
    trigger: On<ApplyShowcase>,
    mut teleports: MessageWriter<OutboundTeleportMessage>,
    mut times: MessageWriter<OutboundSetTimeOfDayMessage>,
    mut look: Query<&mut LocalPlayerLook, With<LocalPlayer>>,
) {
    let Some(showcase) = SHOWCASES.get(trigger.event().0) else {
        return;
    };
    info!("Applying showcase '{}'", showcase.name);

    teleports.write(OutboundTeleportMessage {
        position: showcase.position,
    });

    // the presets were authored as `rotation_y(-(yaw + 90°)) * rotation_x(pitch)`, which is
    // the player's YXZ look with this yaw
    if let Ok(mut look) = look.single_mut() {
        look.yaw = -(showcase.yaw + 90.0).to_radians();
        look.pitch = showcase.pitch.to_radians();
    }

    times.write(OutboundSetTimeOfDayMessage {
        fraction: showcase.time_of_day,
    });
}
