use crate::prelude::*;
use crate::render::{
    pipeline::main_passes::opaque_pass::extract::OpaqueRenderMeshComponent,
    pipeline::main_passes::opaque_pass::startup::{
        OpaquePipelineKey, OpaquePipelines, OpaqueRenderMode,
    },
    types::RenderTransformComponent,
};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    CachedRenderPipelineId, PipelineCache, SpecializedRenderPipelines,
};
use bevy::render::view::{ExtractedView, Msaa};

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
    meshes_query: Query<(
        Entity,
        &OpaqueRenderMeshComponent,
        &RenderTransformComponent,
    )>,
    render_mode: Res<OpaqueRenderMode>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<OpaquePipelines>>,
    opaque_pipelines: Res<OpaquePipelines>,
) {
    for (extracted_view, msaa, mut opaque_phase) in views_query.iter_mut() {
        opaque_phase.items.clear();

        // specialize pipelines for this view's MSAA and HDR settings
        let mesh_key = OpaquePipelineKey {
            msaa_samples: msaa.samples(),
            hdr: extracted_view.hdr,
            mode: *render_mode,
            is_skybox: false,
        };
        opaque_phase.mesh_pipeline_id =
            Some(specialized_pipelines.specialize(&pipeline_cache, &opaque_pipelines, mesh_key));

        let skybox_key = OpaquePipelineKey {
            msaa_samples: msaa.samples(),
            hdr: extracted_view.hdr,
            mode: *render_mode,
            is_skybox: true,
        };
        opaque_phase.skybox_pipeline_id =
            Some(specialized_pipelines.specialize(&pipeline_cache, &opaque_pipelines, skybox_key));

        // collect sortable items for the render pass
        let camera_position = extracted_view.world_from_view.translation();
        let mut sortable_items: Vec<SortableOpaqueItem> =
            Vec::with_capacity(meshes_query.iter().len());
        for (entity, _mesh, transform) in meshes_query.iter() {
            // TODO: Frustum culling here

            let object_position = transform.transform.w_axis.truncate();
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
