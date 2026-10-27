use super::gpu_resources::{UnitCubeMesh, WireframeObjectBuffer};
use crate::prelude::*;
use crate::render::{
    passes::bounding_box::queue::BoundingBoxPhase,
    resources::{CentralCameraViewUniform, EnvironmentUniforms},
};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::{
    IndexFormat, LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    StoreOp,
};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::view::{ViewDepthTexture, ViewTarget};

/// Draws the block bounding boxes. Runs in the `Core3d` schedule of every view that has a
/// bounding box phase.
#[instrument(skip_all, name = "wireframe_render_pass")]
pub fn bounding_box_pass(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture, &BoundingBoxPhase)>,
    wireframe_buffer: Option<Res<WireframeObjectBuffer>>,
    wireframe_mesh: Option<Res<UnitCubeMesh>>,
    view_bind_group: Option<Res<CentralCameraViewUniform>>,
    environment: Option<Res<EnvironmentUniforms>>,
    pipeline_cache: Res<PipelineCache>,
    mut render_context: RenderContext,
) {
    let (view_target, depth_texture, phase) = view.into_inner();

    // INFO: ---------------------------
    //         resource fetching
    // ---------------------------------
    let (Some(wireframe_buffer), Some(wireframe_mesh), Some(view_bind_group), Some(environment)) = (
        wireframe_buffer,
        wireframe_mesh,
        view_bind_group,
        environment,
    ) else {
        return;
    };

    let Some(pipeline_id) = phase.pipeline_id else {
        return;
    };

    let pipeline = pipeline_cache.get_render_pipeline(pipeline_id);
    if pipeline.is_none() {
        return;
    }

    if wireframe_buffer.objects.is_empty() {
        return;
    }

    // INFO: ----------------------
    //         do rendering
    // ----------------------------

    let mut render_pass =
        render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("Wireframe Render Pass"),
                color_attachments: &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_texture.view(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

    // do render
    render_pass.set_pipeline(pipeline.unwrap());

    render_pass.set_bind_group(0, &view_bind_group.bind_group, &[]);
    render_pass.set_bind_group(1, &environment.bind_group, &[]);
    render_pass.set_bind_group(2, &wireframe_buffer.bind_group, &[]);

    render_pass.set_vertex_buffer(0, *wireframe_mesh.vertex_buffer.slice(..));
    render_pass.set_index_buffer(*wireframe_mesh.index_buffer.slice(..), IndexFormat::Uint32);
    render_pass.draw_indexed(
        0..wireframe_mesh.index_count,
        0,
        0..wireframe_buffer.objects.len() as u32,
    );
}
