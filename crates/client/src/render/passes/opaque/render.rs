use crate::prelude::*;
use crate::render::{
    data::RenderMeshStorageResource,
    passes::opaque::{extract::OpaqueRenderMeshComponent, queue::Opaque3dRenderPhase},
    resources::{
        CentralCameraViewUniform, EnvironmentUniforms, TextureArrayUniforms,
        world_uniforms::ChunkStorageManager,
    },
};
use bevy::{
    ecs::prelude::*,
    ecs::query::QueryItem,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment,
            RenderPassDescriptor, StoreOp,
        },
        renderer::RenderContext,
        view::{ViewDepthTexture, ViewTarget},
    },
};

#[derive(Default)]
pub struct OpaquePassRenderNode;

impl ViewNode for OpaquePassRenderNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static Opaque3dRenderPhase,
        &'static ViewDepthTexture,
    );

    #[instrument(skip_all, name = "opaque_render_node")]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, phase, depth_texture): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // INFO: -------------------------------------
        //         collect rendering resources
        // -------------------------------------------
        let (
            Some(mesh_storage),
            Some(view_buffer),
            Some(material_bind_group),
            Some(skybox_params),
            Some(chunk_memory_manager),
            Some(pipeline_cache),
        ) = (
            world.get_resource::<RenderMeshStorageResource>(),
            world.get_resource::<CentralCameraViewUniform>(),
            world.get_resource::<TextureArrayUniforms>(),
            world.get_resource::<EnvironmentUniforms>(),
            world.get_resource::<ChunkStorageManager>(),
            world.get_resource::<PipelineCache>(),
        )
        else {
            return Ok(());
        };

        let Some(skybox_pipeline_id) = phase.skybox_pipeline_id else {
            return Ok(());
        };
        let Some(mesh_pipeline_id) = phase.mesh_pipeline_id else {
            return Ok(());
        };

        let skybox_pipeline = pipeline_cache.get_render_pipeline(skybox_pipeline_id);
        let active_pipeline = pipeline_cache.get_render_pipeline(mesh_pipeline_id);

        if skybox_pipeline.is_none() || active_pipeline.is_none() {
            return Ok(());
        }

        // INFO: --------------------------------
        //         set up the render pass
        // --------------------------------------
        let mut render_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("Opaque Pass"),
                    color_attachments: &[Some(view_target.get_color_attachment())],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: depth_texture.view(),
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(0.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        // INFO: -----------------------------------------
        //         mesh pipeline: iterate and draw
        // -----------------------------------------------
        render_pass.set_pipeline(active_pipeline.unwrap());
        render_pass.set_bind_group(0, &view_buffer.bind_group, &[]);
        render_pass.set_bind_group(1, &skybox_params.bind_group, &[]);
        render_pass.set_bind_group(2, &material_bind_group.bind_group, &[]);
        render_pass.set_bind_group(3, &chunk_memory_manager.bind_group, &[]);

        for item in phase.items.iter() {
            if let Some(render_mesh_comp) = world.get::<OpaqueRenderMeshComponent>(item.entity)
                && let Some(gpu_mesh) = mesh_storage.meshes.get(&render_mesh_comp.mesh_handle.id())
            {
                let object_index = gpu_mesh.slot_index;

                render_pass.draw(
                    0..(gpu_mesh.face_count * 6),
                    object_index..(object_index + 1),
                );
            }
        }

        // INFO: -------------------------
        //         skybox pipeline
        // -------------------------------
        // runs after opaque, filling in empty pixels where depth = 0.0 still
        render_pass.set_pipeline(skybox_pipeline.unwrap());
        render_pass.set_bind_group(0, &view_buffer.bind_group, &[]);
        render_pass.set_bind_group(1, &skybox_params.bind_group, &[]);
        render_pass.draw(0..6, 0..1);

        Ok(())
    }
}
