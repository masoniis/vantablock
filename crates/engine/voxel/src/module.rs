//! Canonical voxel modules: the authoritative geometry of one rigid pose group.
//!
//! A module is a sparse grid of material IDs stored as `4 x 4 x 4` bricks. The grid is
//! `4^levels` voxels on each axis, matching the 64-tree that will be baked from it (five levels
//! cover `1024^3`). Brick-local voxel order is [`crate::mask::cell_index`], the same order used
//! by terminal node children.

use crate::mask::{Mask64, cell_index};
use glam::UVec3;
use std::collections::HashMap;

/// A palette index. `0` is reserved for empty space.
pub type MaterialId = u16;
pub const EMPTY: MaterialId = 0;

/// Voxels per brick edge, which is also the fan-out of a `Node64` per axis.
pub const BRICK_EDGE: u32 = 4;
pub const BRICK_VOXELS: usize = 64;
/// The deepest supported hierarchy: `4^5 = 1024` voxels per axis, matching the 10-bit cell
/// coordinates used by visibility.
pub const MAX_LEVELS: u8 = 5;

/// Materials for one `4 x 4 x 4` brick, indexed by [`cell_index`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Brick {
    pub materials: [MaterialId; BRICK_VOXELS],
}

impl Default for Brick {
    fn default() -> Self {
        Self {
            materials: [EMPTY; BRICK_VOXELS],
        }
    }
}

impl Brick {
    pub fn occupancy(&self) -> Mask64 {
        let mut mask = Mask64::EMPTY;
        for (k, &m) in self.materials.iter().enumerate() {
            if m != EMPTY {
                mask.set(k as u32);
            }
        }
        mask
    }

    pub fn is_empty(&self) -> bool {
        self.materials.iter().all(|&m| m == EMPTY)
    }
}

/// A sparse canonical voxel grid. See the [module docs](self).
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelModule {
    levels: u8,
    bricks: HashMap<UVec3, Brick>,
}

impl VoxelModule {
    /// Creates an empty module of `4^levels` voxels per axis.
    pub fn new(levels: u8) -> Self {
        assert!(
            (1..=MAX_LEVELS).contains(&levels),
            "module levels must be in 1..={MAX_LEVELS}"
        );
        Self {
            levels,
            bricks: HashMap::new(),
        }
    }

    /// The smallest module that holds `extent` voxels per axis.
    pub fn with_extent(extent: u32) -> Self {
        let mut levels = 1;
        while 4u32.pow(levels as u32) < extent {
            levels += 1;
        }
        Self::new(levels)
    }

    pub fn levels(&self) -> u8 {
        self.levels
    }

    /// Voxels per axis.
    pub fn extent(&self) -> u32 {
        4u32.pow(self.levels as u32)
    }

    pub fn in_bounds(&self, p: UVec3) -> bool {
        p.max_element() < self.extent()
    }

    /// Writes a voxel. Writes outside the grid are ignored and return `false`.
    pub fn set(&mut self, p: UVec3, material: MaterialId) -> bool {
        if !self.in_bounds(p) {
            return false;
        }
        let k = brick_cell(p);
        if material == EMPTY {
            if let Some(brick) = self.bricks.get_mut(&(p / BRICK_EDGE)) {
                brick.materials[k] = EMPTY;
                if brick.is_empty() {
                    self.bricks.remove(&(p / BRICK_EDGE));
                }
            }
        } else {
            self.bricks.entry(p / BRICK_EDGE).or_default().materials[k] = material;
        }
        true
    }

    pub fn get(&self, p: UVec3) -> MaterialId {
        self.bricks
            .get(&(p / BRICK_EDGE))
            .map_or(EMPTY, |b| b.materials[brick_cell(p)])
    }

    /// Non-empty bricks keyed by brick coordinate (voxel coordinate / 4).
    pub fn bricks(&self) -> &HashMap<UVec3, Brick> {
        &self.bricks
    }

    /// Inserts a whole brick, used when decoding `.vanta` files.
    pub fn insert_brick(&mut self, coord: UVec3, brick: Brick) {
        assert!(
            (coord * BRICK_EDGE).max_element() < self.extent(),
            "brick outside module"
        );
        if !brick.is_empty() {
            self.bricks.insert(coord, brick);
        }
    }

    pub fn voxel_count(&self) -> u64 {
        self.bricks
            .values()
            .map(|b| b.occupancy().count() as u64)
            .sum()
    }

    /// Iterates every occupied voxel as `(position, material)`.
    pub fn voxels(&self) -> impl Iterator<Item = (UVec3, MaterialId)> + '_ {
        self.bricks.iter().flat_map(|(&coord, brick)| {
            brick.occupancy().iter().map(move |k| {
                let local = UVec3::new(k & 3, (k >> 2) & 3, k >> 4);
                (coord * BRICK_EDGE + local, brick.materials[k as usize])
            })
        })
    }

    /// The occupied region as `(min, max)` voxel coordinates, `max` exclusive.
    pub fn occupied_bounds(&self) -> Option<(UVec3, UVec3)> {
        let mut bounds: Option<(UVec3, UVec3)> = None;
        for (p, _) in self.voxels() {
            bounds = Some(match bounds {
                None => (p, p + UVec3::ONE),
                Some((min, max)) => (min.min(p), max.max(p + UVec3::ONE)),
            });
        }
        bounds
    }
}

fn brick_cell(p: UVec3) -> usize {
    let local = p % BRICK_EDGE;
    cell_index(local.x, local.y, local.z) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_clear() {
        let mut m = VoxelModule::new(2);
        assert_eq!(m.extent(), 16);
        assert!(m.set(UVec3::new(5, 6, 7), 3));
        assert_eq!(m.get(UVec3::new(5, 6, 7)), 3);
        assert_eq!(m.voxel_count(), 1);
        assert!(!m.set(UVec3::new(16, 0, 0), 1));

        m.set(UVec3::new(5, 6, 7), EMPTY);
        assert!(m.bricks().is_empty(), "empty bricks are dropped");
    }

    #[test]
    fn extent_selection() {
        assert_eq!(VoxelModule::with_extent(4).levels(), 1);
        assert_eq!(VoxelModule::with_extent(5).levels(), 2);
        assert_eq!(VoxelModule::with_extent(512).levels(), 5);
    }

    #[test]
    fn occupied_bounds_are_exclusive() {
        let mut m = VoxelModule::new(3);
        m.set(UVec3::new(1, 2, 3), 1);
        m.set(UVec3::new(10, 4, 3), 1);
        assert_eq!(
            m.occupied_bounds(),
            Some((UVec3::new(1, 2, 3), UVec3::new(11, 5, 4)))
        );
    }
}
