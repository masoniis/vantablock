//! Filtered LOD summaries baked for every node.
//!
//! Each summary packs into four `u32` words (the proposed 16-byte payload):
//!
//! | Word | Contents                                                      |
//! | ---- | ------------------------------------------------------------- |
//! | 0    | `f16` solid volume fraction + `f16` axis-X projected coverage |
//! | 1    | `f16` axis-Y coverage + `f16` axis-Z coverage                 |
//! | 2    | 12+12 bit octahedral mean normal + 8 bit normal spread        |
//! | 3    | representative material (low 16 bits), high 16 reserved      |
//!
//! Axis coverage is the exact fraction of the node's face covered by its projected geometry
//! along that axis, computed from full-resolution bitmaps at bake time. Using it as directional
//! opacity is an approximation (see spec section 3.4); the normal and material are approximations
//! too. [`crate::hierarchy::tree64::trace_filtered`] reads them through a [`FootprintFilter`], as
//! does its mirror in `assets/client/shader/voxel/lib/lod.wesl` and `tree64.wesl`.

use crate::module::MaterialId;
use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// A decoded LOD summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodSummary {
    pub volume: f32,
    pub coverage: [f32; 3],
    /// Mean outward surface normal (zero when the node has no exposed faces).
    pub normal: Vec3,
    /// `0` when every exposed face agrees with `normal`, `1` when they cancel out.
    pub spread: f32,
    pub material: MaterialId,
}

impl LodSummary {
    pub fn pack(&self) -> [u32; 4] {
        let w0 = f32_to_f16(self.volume) as u32 | (f32_to_f16(self.coverage[0]) as u32) << 16;
        let w1 = f32_to_f16(self.coverage[1]) as u32 | (f32_to_f16(self.coverage[2]) as u32) << 16;
        let oct = octahedral_encode(self.normal);
        let q = |v: f32| (((v * 0.5 + 0.5).clamp(0.0, 1.0) * 4095.0).round() as u32) & 0xFFF;
        let spread = (self.spread.clamp(0.0, 1.0) * 255.0).round() as u32;
        let w2 = q(oct.x) | q(oct.y) << 12 | spread << 24;
        [w0, w1, w2, self.material as u32]
    }

    pub fn unpack(words: [u32; 4]) -> Self {
        let dq = |v: u32| (v & 0xFFF) as f32 / 4095.0 * 2.0 - 1.0;
        let spread = (words[2] >> 24) as f32 / 255.0;
        Self {
            volume: f16_to_f32(words[0] as u16),
            coverage: [
                f16_to_f32((words[0] >> 16) as u16),
                f16_to_f32(words[1] as u16),
                f16_to_f32((words[1] >> 16) as u16),
            ],
            normal: if spread >= 1.0 {
                Vec3::ZERO
            } else {
                octahedral_decode(Vec2::new(dq(words[2]), dq(words[2] >> 12)))
            },
            spread,
            material: words[3] as MaterialId,
        }
    }
}

impl LodSummary {
    /// The axis-coverage opacity proxy for a ray along `dir`:
    /// `clamp(sum(|dir_axis| * coverage_axis), 0, 1)` with `dir` normalized. A low-cost
    /// approximation (spec section 3.4), not exact arbitrary-angle coverage.
    pub fn opacity(&self, dir: Vec3) -> f32 {
        dir.normalize_or_zero()
            .abs()
            .dot(Vec3::from(self.coverage))
            .clamp(0.0, 1.0)
    }
}

/// Footprint LOD filtering for one ray cone (spec section 3.4).
///
/// The cone's footprint radius at distance `t` is `radius + spread * t` in world units. Traversal
/// stops descending into an occupied node whose cell edge fits inside the footprint's diameter and
/// treats the node's [`LodSummary`] as a surface: the ray hits it when `threshold` falls below the
/// summary's [`opacity`](LodSummary::opacity), and passes through it otherwise. A `threshold` of
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

fn octahedral_encode(n: Vec3) -> Vec2 {
    let l1 = n.x.abs() + n.y.abs() + n.z.abs();
    if l1 == 0.0 {
        return Vec2::ZERO;
    }
    let p = Vec2::new(n.x, n.y) / l1;
    if n.z >= 0.0 {
        p
    } else {
        (Vec2::ONE - Vec2::new(p.y.abs(), p.x.abs())) * Vec2::new(p.x.signum(), p.y.signum())
    }
}

fn octahedral_decode(e: Vec2) -> Vec3 {
    let z = 1.0 - e.x.abs() - e.y.abs();
    let mut xy = e;
    if z < 0.0 {
        xy = (Vec2::ONE - Vec2::new(e.y.abs(), e.x.abs())) * Vec2::new(e.x.signum(), e.y.signum());
    }
    Vec3::new(xy.x, xy.y, z).normalize_or_zero()
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
            normal: Vec3::new(0.3, -0.8, 0.52).normalize(),
            spread: 0.4,
            material: 42,
        };
        let back = LodSummary::unpack(s.pack());
        assert_eq!(back.volume, 0.25);
        assert_eq!(back.coverage, [0.5, 0.75, 1.0]);
        assert_eq!(back.material, 42);
        assert!((back.spread - 0.4).abs() < 0.01);
        assert!(back.normal.dot(s.normal) > 0.999);
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
