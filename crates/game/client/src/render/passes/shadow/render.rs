use crate::prelude::*;
use crate::render::resources::world_uniforms::ChunkGpuStorage;
use crate::render::{
    data::RenderMeshStorageResource,
    passes::opaque::{extract::OpaqueRenderMeshComponent, queue::Opaque3dRenderPhase},
    passes::shadow::gpu_resources::{
        ShadowDepthTextureResource, ShadowPassPipeline, ShadowViewBuffer,
    },
};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    StoreOp,
};
use bevy::render::renderer::{RenderContext, ViewQuery};

/// Renders the opaque meshes into the shadow map. Runs in the `Core3d` schedule of every view
/// that has an opaque phase.
#[instrument(skip_all, name = "shadow_render_pass")]
pub fn shadow_pass(
    view: ViewQuery<&Opaque3dRenderPhase>,
    pipeline_res: Option<Res<ShadowPassPipeline>>,
    shadow_view_buffer: Option<Res<ShadowViewBuffer>>,
    shadow_depth_texture: Option<Res<ShadowDepthTextureResource>>,
    mesh_storage: Option<Res<RenderMeshStorageResource>>,
    gpu_storage: Option<Res<ChunkGpuStorage>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Query<&OpaqueRenderMeshComponent>,
    mut render_context: RenderContext,
) {
    let opaque_phase = view.into_inner();
    let (
        Some(pipeline_res),
        Some(shadow_view_buffer),
        Some(shadow_depth_texture),
        Some(mesh_storage),
        Some(gpu_storage),
    ) = (
        pipeline_res,
        shadow_view_buffer,
        shadow_depth_texture,
        mesh_storage,
        gpu_storage,
    )
    else {
        return;
    };

    let pipeline = pipeline_cache.get_render_pipeline(pipeline_res.pipeline_id);
    if pipeline.is_none() {
        return;
    }

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
                multiview_mask: None,
            });

    render_pass.set_pipeline(pipeline.unwrap());
    render_pass.set_bind_group(0, &shadow_view_buffer.bind_group, &[]);
    render_pass.set_bind_group(1, &gpu_storage.bind_group, &[]);

    // iterate directly over the phase passed into the method
    for item in opaque_phase.items.iter() {
        if let Ok(render_mesh_comp) = render_meshes.get(item.entity)
            && let Some(gpu_mesh) = mesh_storage.meshes.get(&render_mesh_comp.mesh_handle.id())
        {
            let object_index = gpu_mesh.slot_index;

            render_pass.draw(
                0..(gpu_mesh.face_count * 6),
                object_index..(object_index + 1),
            );
        }
    }
}
