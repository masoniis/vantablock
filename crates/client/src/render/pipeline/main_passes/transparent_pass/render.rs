use crate::prelude::*;
use crate::render::pipeline::gpu_resources::world_uniforms::ChunkStorageManager;
use crate::render::pipeline::main_passes::shared_resources::TextureArrayUniforms;
use crate::render::{
    data::RenderMeshStorageResource,
    pipeline::main_passes::{
        shared_resources::{
            CentralCameraViewUniform, EnvironmentUniforms,
            main_depth_texture::MainDepthTextureResource,
        },
        transparent_pass::{
            extract::TransparentRenderMeshComponent, queue::Transparent3dRenderPhase,
            startup::TransparentPipeline,
        },
    },
};
use bevy::ecs::prelude::*;
use bevy::ecs::query::QueryItem;
use bevy::render::render_graph::{NodeRunError, RenderGraphContext, ViewNode};
use bevy::render::render_resource::{
    LoadOp, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp,
};
use bevy::render::renderer::RenderContext;
use bevy::render::view::ViewTarget;

#[derive(Default)]
pub struct TransparentPassRenderNode;

impl ViewNode for TransparentPassRenderNode {
    type ViewQuery = &'static ViewTarget;

    #[instrument(skip_all, name = "transparent_pass_render_node")]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // INFO: -------------------------------------
        //         collect rendering resources
        // -------------------------------------------
        let (
            Some(phase),
            Some(mesh_storage),
            Some(view_buffer),
            Some(material_bind_group),
            Some(depth_texture),
            Some(pipeline_res),
            Some(skybox_params),
            Some(chunk_memory_manager),
            Some(pipeline_cache),
        ) = (
            world.get_resource::<Transparent3dRenderPhase>(),
            world.get_resource::<RenderMeshStorageResource>(),
            world.get_resource::<CentralCameraViewUniform>(),
            world.get_resource::<TextureArrayUniforms>(),
            world.get_resource::<MainDepthTextureResource>(),
            world.get_resource::<TransparentPipeline>(),
            world.get_resource::<EnvironmentUniforms>(),
            world.get_resource::<ChunkStorageManager>(),
            world.get_resource::<PipelineCache>(),
        )
        else {
            return Ok(());
        };

        let pipeline = pipeline_cache.get_render_pipeline(pipeline_res.pipeline_id);
        if pipeline.is_none() {
            return Ok(());
        }

        // INFO: --------------------------------
        //         set up the render pass
        // --------------------------------------

        let mut render_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("Transparent Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: view_target.main_texture_view(),
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load, // Load the existing frame
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Load, // Load the depth buffer
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
        render_pass.set_pipeline(pipeline.unwrap());

        render_pass.set_bind_group(0, &view_buffer.bind_group, &[]);
        render_pass.set_bind_group(1, &skybox_params.bind_group, &[]);
        render_pass.set_bind_group(2, &material_bind_group.bind_group, &[]);
        render_pass.set_bind_group(3, &chunk_memory_manager.bind_group, &[]);

        for item in phase.items.iter() {
            if let Some(render_mesh_comp) = world.get::<TransparentRenderMeshComponent>(item.entity)
                && let Some(gpu_mesh) = mesh_storage.meshes.get(&render_mesh_comp.mesh_handle.id())
            {
                let object_index = gpu_mesh.slot_index;

                render_pass.draw(
                    0..(gpu_mesh.face_count * 6),
                    object_index..(object_index + 1),
                );
            }
        }

        Ok(())
    }
}
