//! Server-owned world objects, replicated to every client.
//!
//! An object is an instance of a canonical asset: which asset, where it sits, and the small set of
//! animation parameters that, with the shared world clock, deterministically produce its pose.
//! Clients never spawn these; they arrive through replication.

use crate::world::assets::VantaAssetId;
use bevy::{math::Affine3A, prelude::*};
use serde::{Deserialize, Serialize};
use voxel::PoseDriver;

/// A placed instance of a canonical asset.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct WorldObject {
    pub asset: VantaAssetId,
}

/// Where an object sits. `scale` multiplies the asset's own voxel size.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ObjectPlacement {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: f32,
}

impl ObjectPlacement {
    /// The transform from the asset's root frame (voxels) to the world (meters).
    pub fn root_to_world(&self, voxel_size: f32) -> Affine3A {
        Affine3A::from_scale_rotation_translation(
            Vec3::splat(self.scale * voxel_size),
            self.rotation,
            self.translation,
        )
    }
}

/// Per-object animation parameters. A shared driver (the world clock) moves every object, but
/// each one responds with its own phase and strength.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ObjectAnimation {
    pub phase: f32,
    pub gust: f32,
}

impl From<ObjectAnimation> for PoseDriver {
    fn from(a: ObjectAnimation) -> Self {
        PoseDriver {
            phase: a.phase,
            gust: a.gust,
        }
    }
}
