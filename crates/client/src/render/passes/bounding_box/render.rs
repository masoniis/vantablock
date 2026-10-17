use super::gpu_resources::{UnitCubeMesh, WireframeObjectBuffer};
use crate::prelude::*;
use crate::render::{
    passes::bounding_box::queue::BoundingBoxPhase,
    resources::{CentralCameraViewUniform, EnvironmentUniforms},
};
use bevy::ecs::prelude::*;
use bevy::ecs::query::QueryItem;
use bevy::render::render_graph::{NodeRunError, RenderGraphContext, ViewNode};
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::{
    IndexFormat, LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    StoreOp,
};
use bevy::render::renderer::RenderContext;
use bevy::render::view::{ViewDepthTexture, ViewTarget};

#[derive(Default)]
pub struct BoundingBoxNode;

impl ViewNode for BoundingBoxNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static BoundingBoxPhase,
    );

    #[instrument(skip_all, name = "wireframe_pass_render_node")]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, depth_texture, phase): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // INFO: ---------------------------
        //         resource fetching
        // ---------------------------------
        let (
            Some(wireframe_buffer),
            Some(wireframe_mesh),
            Some(view_bind_group),
            Some(environment),
            Some(pipeline_cache),
        ) = (
            world.get_resource::<WireframeObjectBuffer>(),
            world.get_resource::<UnitCubeMesh>(),
            world.get_resource::<CentralCameraViewUniform>(),
            world.get_resource::<EnvironmentUniforms>(),
            world.get_resource::<PipelineCache>(),
        )
        else {
            return Ok(());
        };

        let Some(pipeline_id) = phase.pipeline_id else {
            return Ok(());
        };

        let pipeline = pipeline_cache.get_render_pipeline(pipeline_id);
        if pipeline.is_none() {
            return Ok(());
        }

        if wireframe_buffer.objects.is_empty() {
            return Ok(());
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

        Ok(())
    }
}
