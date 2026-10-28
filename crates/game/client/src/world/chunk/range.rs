use bevy::{ecs::prelude::*, math::IVec3};
use common::world::chunk::{ChunkCoord, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK};

/// Marks the entity chunk streaming and meshing are centered on: the local player.
///
/// The player's `ChunkCoord` follows its predicted `LogicalPosition`, the same position the
/// server streams around. (The camera's follows its `GlobalTransform`, which is still zero on
/// the first frame and would request chunks around the origin.)
#[derive(Component, Default, Clone, Copy)]
pub struct ChunkStreamingAnchor;

/// The anchor of chunk streaming and meshing.
pub type StreamingAnchorQuery<'w, 's> =
    Query<'w, 's, &'static ChunkCoord, With<ChunkStreamingAnchor>>;

/// The chunk the streaming anchor is in, if there is one (there is none outside a session).
pub fn streaming_center(anchor: &StreamingAnchorQuery) -> Option<IVec3> {
    anchor.single().ok().map(|coord| coord.pos)
}

/// Run condition: the streaming anchor moved into a different chunk (or just appeared).
pub fn streaming_anchor_changed_chunk(
    anchor: Query<(), (With<ChunkStreamingAnchor>, Changed<ChunkCoord>)>,
) -> bool {
    !anchor.is_empty()
}

/// Whether a chunk coordinate is inside the world's vertical bounds.
pub fn is_in_bounds(coord: IVec3) -> bool {
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&coord.y)
}

/// Whether `coord` is inside the world's vertical bounds and within `radius` chunks of
/// `center` horizontally (chebyshev distance, matching the square loaded area).
pub fn is_within_horizontal_range(center: IVec3, coord: IVec3, radius: i32) -> bool {
    is_in_bounds(coord)
        && (coord.x - center.x).abs() <= radius
        && (coord.z - center.z).abs() <= radius
}

/// Every chunk coordinate for which `is_within_horizontal_range(center, coord, radius)` holds.
pub fn chunks_in_horizontal_range(center: IVec3, radius: i32) -> impl Iterator<Item = IVec3> {
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).flat_map(move |y| {
        (-radius..=radius).flat_map(move |z| {
            (-radius..=radius).map(move |x| IVec3::new(center.x + x, y, center.z + z))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_iterator_and_predicate_agree() {
        let center = IVec3::new(5, 0, -3);
        let radius = 2;

        let coords: Vec<IVec3> = chunks_in_horizontal_range(center, radius).collect();
        let columns = (2 * radius + 1).pow(2);
        let layers = WORLD_MAX_Y_CHUNK - WORLD_MIN_Y_CHUNK + 1;
        assert_eq!(coords.len() as i32, columns * layers);
        assert!(
            coords
                .iter()
                .all(|c| is_within_horizontal_range(center, *c, radius))
        );

        // the anchor's own height never matters, only the world's vertical bounds do
        let high_center = center.with_y(1000);
        assert!(is_within_horizontal_range(
            high_center,
            center + IVec3::X * 2,
            radius
        ));
        assert!(!is_within_horizontal_range(
            center,
            center + IVec3::X * 3,
            radius
        ));
        assert!(!is_within_horizontal_range(
            center,
            center.with_y(WORLD_MAX_Y_CHUNK + 1),
            radius
        ));
    }
}
