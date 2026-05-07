use crate::prelude::*;
use crate::render::passes::transparent::extract::TransparentRenderMeshComponent;
use crate::render::passes::transparent::startup::{TransparentPipeline, TransparentPipelineKey};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    CachedRenderPipelineId, PipelineCache, SpecializedRenderPipelines,
};
use bevy::render::view::{ExtractedView, Msaa};
use bevy::transform::components::GlobalTransform;

#[derive(Debug)]
pub struct PhaseItem {
    pub entity: Entity,
    pub distance: f32, // for sorting back-to-front
}

#[derive(Component, Default, Debug)]
pub struct Transparent3dRenderPhase {
    pub items: Vec<PhaseItem>,
    pub pipeline_id: Option<CachedRenderPipelineId>,
}

/// The system responsible for populating the `Transparent3dRenderPhase` component on each view.
///
/// Performs a query for all entities that have been extracted into the render
/// world and adds them to a list of draw calls for the renderer to consume.
#[instrument(skip_all)]
pub fn queue_and_prepare_transparent_system(
    // input
    mut views_query: Query<(&ExtractedView, &Msaa, &mut Transparent3dRenderPhase)>,
    meshes_query: Query<(Entity, &TransparentRenderMeshComponent, &GlobalTransform)>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<TransparentPipeline>>,
    transparent_pipeline: Res<TransparentPipeline>,
) {
    for (view, msaa, mut transparent_phase) in views_query.iter_mut() {
        transparent_phase.items.clear();

        // specialize pipeline for this view
        let key = TransparentPipelineKey {
            msaa_samples: msaa.samples(),
            hdr: view.hdr,
        };
        transparent_phase.pipeline_id =
            Some(specialized_pipelines.specialize(&pipeline_cache, &transparent_pipeline, key));

        let camera_position: Vec3 = view.world_from_view.translation();

        // collect sortable items for the render pass
        let mut sortable_items: Vec<PhaseItem> = Vec::with_capacity(meshes_query.iter().len());
        for (entity, _mesh, transform) in meshes_query.iter() {
            let object_position = transform.translation();
            let distance_from_camera = (object_position - camera_position).length_squared();

            sortable_items.push(PhaseItem {
                distance: distance_from_camera,
                entity,
            });
        }

        // sort BACK-TO-FRONT for proper transparency blending (furthest items first).
        sortable_items.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap());

        // populate the phase buffer in correct sorted order
        transparent_phase.items = sortable_items;
    }
}
