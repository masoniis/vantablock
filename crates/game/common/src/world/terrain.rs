//! Settings a world is generated with, chosen when it is created.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The terrain shapers the server can generate with.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TerrainShaperKind {
    Superflat,
    #[default]
    Sinwave,
    Noisy,
    Realistic,
    Simplex3d,
}

impl TerrainShaperKind {
    pub const ALL: [Self; 5] = [
        Self::Superflat,
        Self::Sinwave,
        Self::Noisy,
        Self::Realistic,
        Self::Simplex3d,
    ];

    /// A short name for menus.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Superflat => "Superflat",
            Self::Sinwave => "Sine waves",
            Self::Noisy => "Noisy",
            Self::Realistic => "Realistic",
            Self::Simplex3d => "3D simplex",
        }
    }
}

/// How a world's terrain is generated. The server reads it at startup; a singleplayer world
/// gets it from the new world screen.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct WorldGenSettings {
    pub shaper: TerrainShaperKind,
}
