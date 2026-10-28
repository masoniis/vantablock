use crate::prelude::*;
use crate::render::{
    data::RenderMeshStorageResource,
    passes::transparent::{
        extract::TransparentRenderMeshComponent, queue::Transparent3dRenderPhase,
    },
    resources::TextureArrayUniforms,
    resources::world_uniforms::ChunkGpuStorage,
    resources::{CentralCameraViewUniform, EnvironmentUniforms},
};
use bevy::{
    ecs::prelude::*,
    render::render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
        StoreOp,
    },
    render::renderer::{RenderContext, ViewQuery},
    render::view::{ViewDepthTexture, ViewTarget},
};

/// Draws the transparent meshes. Runs in the `Core3d` schedule of every view that has a
/// transparent phase.
#[instrument(skip_all, name = "transparent_render_pass")]
pub fn transparent_pass(
    view: ViewQuery<(&ViewTarget, &Transparent3dRenderPhase, &ViewDepthTexture)>,
    mesh_storage: Option<Res<RenderMeshStorageResource>>,
    view_buffer: Option<Res<CentralCameraViewUniform>>,
    material_bind_group: Option<Res<TextureArrayUniforms>>,
    skybox_params: Option<Res<EnvironmentUniforms>>,
    gpu_storage: Option<Res<ChunkGpuStorage>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Query<&TransparentRenderMeshComponent>,
    mut render_context: RenderContext,
) {
    let (view_target, phase, depth_texture) = view.into_inner();

    // INFO: -------------------------------------
    //         collect rendering resources
    // -------------------------------------------
    let (
        Some(mesh_storage),
        Some(view_buffer),
        Some(material_bind_group),
        Some(skybox_params),
        Some(gpu_storage),
    ) = (
        mesh_storage,
        view_buffer,
        material_bind_group,
        skybox_params,
        gpu_storage,
    )
    else {
        return;
    };

    let Some(pipeline_id) = phase.pipeline_id else {
        return;
    };

    let active_pipeline = pipeline_cache.get_render_pipeline(pipeline_id);

    if active_pipeline.is_none() {
        return;
    }

    // INFO: --------------------------------
    //         set up the render pass
    // --------------------------------------
    let mut render_pass =
        render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("Transparent Pass"),
                color_attachments: &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_texture.view(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

    // INFO: -----------------------------------------
    //         mesh pipeline: iterate and draw
    // -----------------------------------------------
    render_pass.set_pipeline(active_pipeline.unwrap());
    render_pass.set_bind_group(0, &view_buffer.bind_group, &[]);
    render_pass.set_bind_group(1, &skybox_params.bind_group, &[]);
    render_pass.set_bind_group(2, &material_bind_group.bind_group, &[]);
    render_pass.set_bind_group(3, &gpu_storage.bind_group, &[]);

    for item in phase.items.iter() {
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
