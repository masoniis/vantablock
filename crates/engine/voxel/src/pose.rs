//! Animation state for pose groups.
//!
//! The authoritative input is small: a [`PoseDriver`] per instance (replicated by the server)
//! plus the shared world time. Evaluating it is deterministic, so every peer derives the same
//! [`PoseState`] without streaming per-group transforms. Canonical voxels never move; only pose
//! frames do.

use crate::assembly::{CanonicalAssembly, Motion};
use std::f32::consts::TAU;

/// One angle per pose group (static groups hold `0`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoseState {
    pub angles: Vec<f32>,
}

impl PoseState {
    pub fn rest(assembly: &CanonicalAssembly) -> Self {
        Self {
            angles: vec![0.0; assembly.groups.len()],
        }
    }

    pub fn angle(&self, group: usize) -> f32 {
        self.angles.get(group).copied().unwrap_or(0.0)
    }

    /// Whether every group is inside its motion domain. Poses outside it are not covered by the
    /// template's baked envelopes and must take the per-instance fallback path.
    pub fn is_within_envelope(&self, assembly: &CanonicalAssembly) -> bool {
        self.angles.len() == assembly.groups.len()
            && assembly
                .groups
                .iter()
                .zip(&self.angles)
                .all(|(g, &a)| g.motion.allows(a))
    }
}

/// Per-instance animation parameters: a shared wind time drives every instance, but each
/// instance and group responds with its own phase and frequency.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PoseDriver {
    pub phase: f32,
    /// Gust strength in `0..=1`, the fraction of each hinge's range that is used.
    pub gust: f32,
}

impl PoseDriver {
    pub fn evaluate(&self, assembly: &CanonicalAssembly, time: f32) -> PoseState {
        let gust = self.gust.clamp(0.0, 1.0);
        let angles = assembly
            .groups
            .iter()
            .enumerate()
            .map(|(i, group)| match group.motion {
                Motion::Static => 0.0,
                Motion::Hinge {
                    min_angle,
                    max_angle,
                    response_hz,
                    ..
                } => {
                    let mid = (min_angle + max_angle) * 0.5;
                    let half = (max_angle - min_angle) * 0.5;
                    let freq = response_hz * (0.85 + 0.3 * unit_hash(i as u32));
                    let wave = (TAU * freq * time + self.phase + i as f32 * 1.618).sin();
                    (mid + half * gust * wave).clamp(min_angle, max_angle)
                }
                Motion::Spin { rate, .. } => (rate * time + self.phase).rem_euclid(TAU),
            })
            .collect();
        PoseState { angles }
    }
}

/// A deterministic hash of `x` into `0..1`.
pub fn unit_hash(x: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}
