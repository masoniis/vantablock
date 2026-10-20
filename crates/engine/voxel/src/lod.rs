//! Filtered LOD summaries baked for every node.
//!
//! Each summary packs into four `u32` words (the proposed 16-byte payload):
//!
//! | Word | Contents                                                                   |
//! | ---- | -------------------------------------------------------------------------- |
//! | 0    | `f16` SGGX axis-X coverage `c_x` + `f16` axis-Y coverage `c_y`             |
//! | 1    | `f16` axis-Z coverage `c_z` + representative material (high 16 bits)      |
//! | 2    | SGGX correlations `xy`, `xz`, `yz`: 10 bit offset snorm each (2 bits spare) |
//! | 3    | occupied box, 4 bits per bound (24 bits) + 8 bit unorm solid volume fraction |
//!
//! The occupied box bounds the node's voxels in sixteenths of its edge (rounded outward: exact
//! for nodes up to 16 voxels on a side), so a filtered hit fills only the part of the node its
//! voxels reach (low 12 bits: the minimum corner's x, y, z; next 12: the maximum corner's, minus
//! one). Words 0 to 2 are an SGGX tensor `S_aa = c_a^2`, `S_ab = r_ab c_a c_b`
//! ([`LodSummary::tensor`]) least-squares fitted at bake time to the node's measured angular
//! coverage ([`crate::angular::BakeSetup::tensor`]): the chance that a ray entering the occupied
//! box hits a voxel, so `sigma(w) = sqrt(w^T S w)` is the opacity of a filtered hit. (The spec's
//! coverage is relative to the node's face; that fraction is `c_a` times the box face's share of
//! the node face, which shading uses.) Using it as directional opacity is an approximation (spec
//! section 3.4); the material is one too. [`crate::hierarchy::tree64::trace_filtered`] reads them
//! through a [`FootprintFilter`], as does its mirror in `assets/client/shader/voxel/lib/lod.wesl`
//! and `tree64.wesl`.

use crate::{angular::Sggx, module::MaterialId};
use glam::Vec3;
use std::collections::HashMap;

/// A decoded LOD summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodSummary {
    pub volume: f32,
    /// The fitted SGGX tensor's coverage of the occupied box along X, Y, and Z (`sqrt(S_aa)`,
    /// near the exact projected coverage of the box's faces but fitted over every direction).
    pub coverage: [f32; 3],
    /// The fitted SGGX tensor's correlations `xy`, `xz`, `yz` (`S_ab / (c_a c_b)`, in `[-1, 1]`).
    pub correlation: [f32; 3],
    pub material: MaterialId,
    /// The occupied box's minimum and maximum corners as fractions of the node's edge, in
    /// sixteenths (packing rounds outward).
    pub occupied: [Vec3; 2],
}

/// The occupied box's resolution: sixteenths of a node's edge.
pub const OCCUPIED_STEPS: f32 = 16.0;

impl LodSummary {
    pub fn pack(&self) -> [u32; 4] {
        let w0 = f32_to_f16(self.coverage[0]) as u32 | (f32_to_f16(self.coverage[1]) as u32) << 16;
        let w1 = f32_to_f16(self.coverage[2]) as u32 | (self.material as u32) << 16;
        let [rxy, rxz, ryz] = self.correlation.map(correlation_encode);
        let w2 = rxy | rxz << 10 | ryz << 20;
        let lo = (self.occupied[0] * OCCUPIED_STEPS)
            .floor()
            .clamp(Vec3::ZERO, Vec3::splat(15.0))
            .as_uvec3();
        let hi = ((self.occupied[1] * OCCUPIED_STEPS).ceil() - 1.0)
            .clamp(Vec3::ZERO, Vec3::splat(15.0))
            .as_uvec3()
            .max(lo);
        let volume = (self.volume.clamp(0.0, 1.0) * 255.0).round() as u32;
        let w3 = lo.x | lo.y << 4 | lo.z << 8 | hi.x << 12 | hi.y << 16 | hi.z << 20 | volume << 24;
        [w0, w1, w2, w3]
    }

    pub fn unpack(words: [u32; 4]) -> Self {
        let nibble = |shift: u32| ((words[3] >> shift) & 0xF) as f32;
        Self {
            volume: (words[3] >> 24) as f32 / 255.0,
            coverage: [
                f16_to_f32(words[0] as u16),
                f16_to_f32((words[0] >> 16) as u16),
                f16_to_f32(words[1] as u16),
            ],
            correlation: [0, 10, 20].map(|shift| correlation_decode(words[2] >> shift)),
            material: (words[1] >> 16) as MaterialId,
            occupied: [
                Vec3::new(nibble(0), nibble(4), nibble(8)) / OCCUPIED_STEPS,
                (Vec3::new(nibble(12), nibble(16), nibble(20)) + 1.0) / OCCUPIED_STEPS,
            ],
        }
    }

    /// The box a filtered hit fills: the occupied box of a node whose minimum corner is `lo`,
    /// `size` voxels on a side. Exact in `f32` (sizes are powers of four).
    pub fn filtered_box(&self, lo: Vec3, size: f32) -> (Vec3, Vec3) {
        (lo + self.occupied[0] * size, lo + self.occupied[1] * size)
    }
}

impl LodSummary {
    /// The fitted SGGX tensor the record stores.
    pub fn tensor(&self) -> Sggx {
        Sggx::from_axes_and_correlations(self.coverage, self.correlation)
    }

    /// The fitted tensor's opacity for a ray along `dir` that enters the occupied box:
    /// `min(sqrt(max(w^T S w, 0)), 1)` with `w` the normalized `dir` ([`Self::tensor`], evaluated
    /// as `lod_opacity` in `lod.wesl` does). An approximation of the node's angular coverage (spec
    /// section 3.4); the spec's `L1` axis proxy overestimates it everywhere (see
    /// [`crate::angular`]).
    pub fn opacity(&self, dir: Vec3) -> f32 {
        let a = dir.normalize_or_zero() * Vec3::from(self.coverage);
        let r = self.correlation;
        let q = a.dot(a) + 2.0 * (a.x * a.y * r[0] + a.x * a.z * r[1] + a.y * a.z * r[2]);
        q.max(0.0).sqrt().min(1.0)
    }
}

/// Footprint LOD filtering for one ray cone (spec section 3.4).
///
/// The cone's footprint radius at distance `t` is `radius + spread * t` in world units. Traversal
/// stops descending into an occupied node whose cell edge fits inside the footprint's diameter and
/// treats the node's [`LodSummary`] as a surface filling its occupied box: a ray entering the box
/// hits it when `threshold` falls below the summary's [`opacity`](LodSummary::opacity), and passes
/// through it otherwise. A `threshold` of
/// `0.5` is a deterministic cutoff; a spatially and temporally stable random value in `[0, 1)`
/// gives stochastic transmission.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootprintFilter {
    pub radius: f32,
    pub spread: f32,
    pub threshold: f32,
}

impl FootprintFilter {
    /// Whether a cell `size` voxels on a side, entered at `t`, fits inside the footprint of a ray
    /// whose direction has length `voxels_per_unit` (rays keep `t` in world units, see
    /// [`crate::math`]).
    pub fn covers(&self, size: f32, t: f32, voxels_per_unit: f32) -> bool {
        size <= 2.0 * (self.radius + self.spread * t) * voxels_per_unit
    }
}

/// Deduplicated LOD records. Identical subtrees produce identical summaries, which must share an
/// index so their node headers (and therefore their child blocks) can deduplicate too.
#[derive(Default)]
pub struct LodTable {
    records: Vec<[u32; 4]>,
    dedup: HashMap<[u32; 4], u32>,
}

impl LodTable {
    pub fn push(&mut self, record: [u32; 4]) -> u32 {
        *self.dedup.entry(record).or_insert_with(|| {
            self.records.push(record);
            (self.records.len() - 1) as u32
        })
    }

    pub fn get(&self, index: u32) -> LodSummary {
        LodSummary::unpack(self.records[index as usize])
    }

    pub fn records(&self) -> &[[u32; 4]] {
        &self.records
    }

    pub fn finish_dedup(&mut self) {
        self.dedup = HashMap::new();
    }
}

/// IEEE half precision encoding with round-to-nearest-even, matching `pack2x16float`.
pub fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xFF) as i32;
    let mant = x & 0x7F_FFFF;

    if exp == 0xFF {
        return sign | 0x7C00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1F {
        return sign | 0x7C00;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = mant | 0x80_0000;
        let shift = (14 - e) as u32;
        let half = m >> shift;
        let rem = m & ((1 << shift) - 1);
        let halfway = 1 << (shift - 1);
        let round = (rem > halfway || (rem == halfway && half & 1 == 1)) as u32;
        return sign | (half + round) as u16;
    }
    let mut h = ((e as u32) << 10) | (mant >> 13);
    let rem = mant & 0x1FFF;
    if rem > 0x1000 || (rem == 0x1000 && h & 1 == 1) {
        h += 1;
    }
    sign | h as u16
}

pub fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;
    let bits = match (exp, mant) {
        (0, 0) => sign,
        (0, _) => {
            // subnormal: renormalize
            let mut e = 127 - 15 + 1;
            let mut m = mant;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            sign | (e << 23) | ((m & 0x3FF) << 13)
        }
        (0x1F, _) => sign | 0x7F80_0000 | (mant << 13),
        _ => sign | ((exp + 127 - 15) << 23) | (mant << 13),
    };
    f32::from_bits(bits)
}

/// A correlation in `[-1, 1]` as a 10 bit offset snorm (`0` is exact).
fn correlation_encode(r: f32) -> u32 {
    ((r.clamp(-1.0, 1.0) * 511.0).round() + 511.0) as u32
}

fn correlation_decode(bits: u32) -> f32 {
    ((bits & 0x3FF) as f32 - 511.0) / 511.0
}

/// `tensor` as an LOD record stores it: the axis coverages as `f16` and the correlations as 10
/// bit snorms.
pub fn stored_tensor(tensor: Sggx) -> Sggx {
    let axis = [tensor.0[0], tensor.0[1], tensor.0[2]].map(|v| f16_to_f32(f32_to_f16(v.sqrt())));
    let r = tensor
        .correlations()
        .map(|r| correlation_decode(correlation_encode(r)));
    Sggx::from_axes_and_correlations(axis, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_known_values() {
        assert_eq!(f32_to_f16(0.0), 0x0000);
        assert_eq!(f32_to_f16(-0.0), 0x8000);
        assert_eq!(f32_to_f16(1.0), 0x3C00);
        assert_eq!(f32_to_f16(0.5), 0x3800);
        assert_eq!(f32_to_f16(65504.0), 0x7BFF);
        assert_eq!(f32_to_f16(1e6), 0x7C00);
        // smallest subnormal
        assert_eq!(f32_to_f16(5.960_464_5e-8), 0x0001);
        // 1 + 2^-11 is exactly halfway and rounds to even (down)
        assert_eq!(f32_to_f16(1.0 + 2f32.powi(-11)), 0x3C00);
    }

    #[test]
    fn half_round_trips_unit_values() {
        for i in 0..=10_000 {
            let v = i as f32 / 10_000.0;
            let back = f16_to_f32(f32_to_f16(v));
            assert!((back - v).abs() <= v.max(6.1e-5) * 1e-3, "{v} -> {back}");
        }
        for h in [0x0001u16, 0x03FF, 0x0400, 0x3C00, 0x7BFF] {
            assert_eq!(f32_to_f16(f16_to_f32(h)), h);
        }
    }

    #[test]
    fn summaries_round_trip() {
        let s = LodSummary {
            volume: 0.25,
            coverage: [0.5, 0.75, 1.0],
            correlation: [0.3, -1.0, 0.0],
            material: 42,
            occupied: [Vec3::new(0.0, 0.25, 0.5), Vec3::new(1.0, 0.3, 0.5625)],
        };
        let back = LodSummary::unpack(s.pack());
        assert!((back.volume - 0.25).abs() < 0.5 / 255.0);
        assert_eq!(back.coverage, [0.5, 0.75, 1.0]);
        assert_eq!(back.material, 42);
        // exact sixteenths survive, others round outward
        assert_eq!(
            back.occupied,
            [Vec3::new(0.0, 0.25, 0.5), Vec3::new(1.0, 0.3125, 0.5625)]
        );
        assert!((back.correlation[0] - 0.3).abs() < 0.5 / 511.0);
        assert_eq!(&back.correlation[1..], [-1.0, 0.0]);
    }

    #[test]
    fn identical_records_share_an_index() {
        let mut table = LodTable::default();
        let a = table.push([1, 2, 3, 4]);
        let b = table.push([1, 2, 3, 4]);
        let c = table.push([1, 2, 3, 5]);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(table.records().len(), 2);
    }
}
