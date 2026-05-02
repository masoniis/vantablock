use crate::{
    input::systems::toggle_opaque_wireframe::OpaqueRenderMode,
    prelude::*,
    render::passes::opaque::{
        extract::OpaqueRenderMeshComponent,
        pipeline::{Opaque3dPipeline, Opaque3dPipelineKey},
        skybox::{OpaqueSkyboxPipeline, OpaqueSkyboxPipelineKey},
    },
};
use bevy::{
    ecs::prelude::*,
    render::render_resource::{CachedRenderPipelineId, PipelineCache, SpecializedRenderPipelines},
    render::view::{ExtractedView, Msaa},
    transform::components::GlobalTransform,
};

#[derive(Debug)]
pub struct PhaseItem {
    pub entity: Entity,
    pub distance: f32, // for sorting front-to-back
}

#[derive(Component, Default, Debug)]
pub struct Opaque3dRenderPhase {
    pub items: Vec<PhaseItem>,
    pub mesh_pipeline_id: Option<CachedRenderPipelineId>,
    pub skybox_pipeline_id: Option<CachedRenderPipelineId>,
}

// A temporary struct to hold all the data we need for sorting
struct SortableOpaqueItem {
    distance: f32,
    entity: Entity,
}

/// The system responsible for populating the `Opaque3dRenderPhase` components on each view.
///
/// Performs a query for all entities that have been extracted into the render
/// world and adds them to a list of draw calls for the renderer to consume.
#[instrument(skip_all)]
pub fn queue_opaque_system(
    // input
    mut views_query: Query<(&ExtractedView, &Msaa, &mut Opaque3dRenderPhase)>,
    meshes_query: Query<(Entity, &OpaqueRenderMeshComponent, &GlobalTransform)>,
    render_mode: Res<OpaqueRenderMode>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_world_pipelines: ResMut<SpecializedRenderPipelines<Opaque3dPipeline>>,
    world_pipelines: Res<Opaque3dPipeline>,
    mut specialized_skybox_pipelines: ResMut<SpecializedRenderPipelines<OpaqueSkyboxPipeline>>,
    skybox_pipelines: Res<OpaqueSkyboxPipeline>,
) {
    for (extracted_view, msaa, mut opaque_phase) in views_query.iter_mut() {
        opaque_phase.items.clear();

        // specialize pipelines for this view's MSAA and HDR settings
        let mesh_key = Opaque3dPipelineKey {
            msaa_samples: msaa.samples(),
            hdr: extracted_view.hdr,
            mode: *render_mode,
        };
        opaque_phase.mesh_pipeline_id = Some(specialized_world_pipelines.specialize(
            &pipeline_cache,
            &world_pipelines,
            mesh_key,
        ));

        let skybox_key = OpaqueSkyboxPipelineKey {
            msaa_samples: msaa.samples(),
            hdr: extracted_view.hdr,
        };
        opaque_phase.skybox_pipeline_id = Some(specialized_skybox_pipelines.specialize(
            &pipeline_cache,
            &skybox_pipelines,
            skybox_key,
        ));

        // collect sortable items for the render pass
        let camera_position = extracted_view.world_from_view.translation();
        let mut sortable_items: Vec<SortableOpaqueItem> =
            Vec::with_capacity(meshes_query.iter().len());
        for (entity, _mesh, transform) in meshes_query.iter() {
            // TODO: frustum culling
            let object_position = transform.translation();
            let distance_from_camera = (object_position - camera_position).length_squared();

            sortable_items.push(SortableOpaqueItem {
                distance: distance_from_camera,
                entity,
            });
        }

        // sort by front to back (which optimizes early z-culling in the GPU)
        sortable_items.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        // populate the phase and object buffer in correct sorted order
        for item in sortable_items {
            opaque_phase.items.push(PhaseItem {
                entity: item.entity,
                distance: item.distance,
            });
        }
    }
}
