use crate::prelude::*;
use crate::render::pipeline::main_passes::transparent_pass::extract::TransparentRenderMeshComponent;
use crate::render::types::RenderTransformComponent;
use bevy::ecs::prelude::*;
use bevy::render::view::ExtractedView;

#[derive(Debug)]
pub struct PhaseItem {
    pub entity: Entity,
    pub distance: f32, // for sorting back-to-front
}

#[derive(Resource, Default)]
pub struct Transparent3dRenderPhase {
    pub items: Vec<PhaseItem>,
}

/// The system responsible for populating the `Transparent3dRenderPhase`.
///
/// Performs a query for all entities that have been extracted into the render
/// world and adds them to a list of draw calls for the renderer to consume.
#[instrument(skip_all)]
pub fn queue_and_prepare_transparent_system(
    // Input
    views_query: Query<&ExtractedView>,
    meshes_query: Query<(
        Entity,
        &TransparentRenderMeshComponent,
        &RenderTransformComponent,
    )>,

    // Output
    mut transparent_phase: ResMut<Transparent3dRenderPhase>,
) {
    transparent_phase.items.clear();

    let Ok(view) = views_query.single() else {
        return;
    };
    let camera_position: Vec3 = view.world_from_view.translation();

    // collect sortable items for the render pass
    let mut sortable_items: Vec<PhaseItem> = Vec::with_capacity(meshes_query.iter().len());
    for (entity, _mesh, transform) in meshes_query.iter() {
        // TODO: Bevy Frustum culling here using view.frustum or Bevy's ViewVisibility

        let object_position = transform.transform.w_axis.truncate();
        let distance_from_camera = (object_position - camera_position).length_squared();

        sortable_items.push(PhaseItem {
            distance: distance_from_camera,
            entity,
        });
    }

    // Sort BACK-TO-FRONT for proper transparency blending (furthest items first).
    // Note: b.cmp(a) ensures the largest distances are at the beginning of the list.
    sortable_items.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap());

    // populate the phase buffer in correct sorted order
    transparent_phase.items = sortable_items;
}
