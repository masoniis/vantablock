use crate::simulation::chunk::{WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK};
use bevy::math::IVec3;

/// Offsets to find the 26 direct neighbors of a chunk.
pub const NEIGHBOR_OFFSETS: [IVec3; 26] = {
    let mut offsets = [IVec3::ZERO; 26];

    let mut index = 0;
    let mut x = -1;
    while x <= 1 {
        let mut y = -1;
        while y <= 1 {
            let mut z = -1;
            while z <= 1 {
                if x != 0 || y != 0 || z != 0 {
                    offsets[index] = IVec3::new(x, y, z);
                    index += 1;
                }
                z += 1;
            }
            y += 1;
        }
        x += 1;
    }

    offsets
};

/// Determines if a coord is in bounds
pub fn is_in_bounds(coord: IVec3) -> bool {
    let pos_y = coord.y;
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&pos_y)
}
