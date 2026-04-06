use crate::prelude::*;
use crate::render::passes::gpu_resources::world_uniforms::ChunkStorageManager;
use crate::render::{
    global_extract::RenderMeshStorageResource,
    passes::main_passes::opaque_pass::{
        extract::OpaqueRenderMeshComponent, queue::Opaque3dRenderPhase,
    },
    passes::shadow_pass::gpu_resources::{
        ShadowDepthTextureResource, ShadowPassPipeline, ShadowViewBuffer,
    },
};

use bevy::ecs::prelude::*;
use bevy::ecs::query::QueryItem;

use bevy::render::render_graph::{NodeRunError, RenderGraphContext, ViewNode};
use bevy::render::render_resource::{
    LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    StoreOp,
};
use bevy::render::renderer::RenderContext;

#[derive(Default)]
pub struct ShadowRenderPassNode;

impl ViewNode for ShadowRenderPassNode {
    type ViewQuery = ();

    #[instrument(skip_all, name = "shadow_pass_render_node")]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        _view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // INFO: --------------------------------------
        //          collect rendering resources
        // --------------------------------------------

        let (
            // shadow-specific stuff
            Some(pipeline_res),
            Some(shadow_view_buffer),
            Some(shadow_depth_texture),
            // opaque mesh to base shadow depth on
            Some(phase),
            Some(mesh_storage),
            Some(chunk_memory_manager),
            Some(pipeline_cache),
        ) = (
            world.get_resource::<ShadowPassPipeline>(),
            world.get_resource::<ShadowViewBuffer>(),
            world.get_resource::<ShadowDepthTextureResource>(),
            world.get_resource::<Opaque3dRenderPhase>(),
            world.get_resource::<RenderMeshStorageResource>(),
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
                    label: Some("Shadow Map Render Pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &shadow_depth_texture.view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        // INFO: -------------------------------------------
        //         shadow pipeline: iterate and draw
        // -------------------------------------------------

        render_pass.set_pipeline(pipeline.unwrap());

        render_pass.set_bind_group(0, &shadow_view_buffer.bind_group, &[]);
        render_pass.set_bind_group(1, &chunk_memory_manager.bind_group, &[]);

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

        Ok(())
    }
}
