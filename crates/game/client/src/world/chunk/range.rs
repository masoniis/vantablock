use bevy::{ecs::prelude::*, math::IVec3, prelude::*};
use common::world::chunk::{ChunkCoord, WORLD_MAX_Y_CHUNK, WORLD_MIN_Y_CHUNK};

/// The cameras chunk streaming and meshing are centered on.
pub type ChunkCameraQuery<'w, 's> =
    Query<'w, 's, (&'static Camera, &'static ChunkCoord), With<Camera3d>>;

/// The chunk coordinate of the active 3D camera, if there is one.
pub fn active_camera_chunk(camera_query: &ChunkCameraQuery) -> Option<IVec3> {
    camera_query
        .iter()
        .find(|(camera, _)| camera.is_active)
        .map(|(_, coord)| coord.pos)
}

/// Run condition: the active 3D camera moved into a different chunk this frame.
pub fn active_camera_changed_chunk(
    camera_query: Query<(&Camera, &ChunkCoord), (With<Camera3d>, Changed<ChunkCoord>)>,
) -> bool {
    camera_query.iter().any(|(camera, _)| camera.is_active)
}

/// Whether `coord` is inside the world's vertical bounds and within `radius` chunks of
/// `center` horizontally (chebyshev distance, matching the square loaded area).
pub fn is_within_horizontal_range(center: IVec3, coord: IVec3, radius: i32) -> bool {
    (WORLD_MIN_Y_CHUNK..=WORLD_MAX_Y_CHUNK).contains(&coord.y)
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

        // the camera's own height never matters, only the world's vertical bounds do
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
