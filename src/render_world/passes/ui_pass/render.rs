use crate::{
    prelude::*,
    render_world::passes::{
        core::{RenderContext, RenderNode},
        ui_pass::{
            gpu_resources::{
                GlyphonAtlasResource, GlyphonRendererResource, GlyphonViewportResource,
                UiMaterialBuffer, UiObjectBuffer, UiPipeline,
            },
            queue::{PreparedUiBatches, UiRenderBatch},
        },
    },
};
use bevy::ecs::world::World;

use super::gpu_resources::{view_binding::UiViewBuffer, ScreenQuadResource};

pub struct UiRenderPassNode;
impl RenderNode for UiRenderPassNode {
    #[instrument(skip_all, name = "ui_pass_render_node")]
    fn run(&mut self, render_context: &mut RenderContext, world: &World) {
        // INFO: ---------------------------
        //         resource fetching
        // ---------------------------------

        let ui_phase = world.get_resource::<PreparedUiBatches>().unwrap();
        let pipeline = world.get_resource::<UiPipeline>().unwrap();
        let quad = world.get_resource::<ScreenQuadResource>().unwrap();
        let view_bind_group = world.get_resource::<UiViewBuffer>().unwrap();
        let material_buffer = world.get_resource::<UiMaterialBuffer>().unwrap();
        let object_buffer = world.get_resource::<UiObjectBuffer>().unwrap();

        let text_atlas = world.get_resource::<GlyphonAtlasResource>().unwrap();
        let glyphon_viewport = world.get_resource::<GlyphonViewportResource>().unwrap();
        let text_renderer = world.get_resource::<GlyphonRendererResource>().unwrap();

        // INFO: ----------------------
        //         render logic
        // ----------------------------

        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("UI Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_context.surface_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        let mut is_panel_pipeline_set = false;

        for batch in &ui_phase.batches {
            match batch {
                UiRenderBatch::Panel(panel_batch) => {
                    if !is_panel_pipeline_set {
                        render_pass.set_pipeline(pipeline);
                        render_pass.set_bind_group(0, &view_bind_group.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, quad.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(
                            quad.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        render_pass.set_bind_group(2, &object_buffer.bind_group, &[]);
                        is_panel_pipeline_set = true;
                    }

                    let material_offset = panel_batch.material_index * material_buffer.stride;
                    render_pass.set_bind_group(1, &material_buffer.bind_group, &[material_offset]);
                    render_pass.draw_indexed(
                        0..quad.index_count,
                        0,
                        panel_batch.first_instance
                            ..panel_batch.first_instance + panel_batch.instance_count,
                    );
                }
                UiRenderBatch::Text(_) => {
                    is_panel_pipeline_set = false;

                    // triggers the render for the **next** text batch
                    // (batches set in the prepare_glyphon_text system)
                    text_renderer
                        .render(text_atlas, glyphon_viewport, &mut render_pass)
                        .unwrap();
                }
            }
        }
    }
}
