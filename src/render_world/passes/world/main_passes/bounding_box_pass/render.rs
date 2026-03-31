use super::gpu_resources::WireframeObjectBuffer;
use crate::{
    prelude::*,
    render_world::passes::{
        core::{RenderContext, RenderNode},
        world::main_passes::{
            bounding_box_pass::gpu_resources::{
                unit_cube_mesh::UnitCubeMesh, wireframe_pipeline::*,
            },
            shared_resources::main_depth_texture::MainDepthTextureResource,
            shared_resources::{CentralCameraViewUniform, EnvironmentUniforms},
        },
    },
};
use bevy::ecs::prelude::*;

pub struct BoundingBoxNode;

impl RenderNode for BoundingBoxNode {
    #[instrument(skip_all, name = "wireframe_pass_render_node")]
    fn run(&mut self, render_context: &mut RenderContext, world: &World) {
        // INFO: ---------------------------
        //         resource fetching
        // ---------------------------------

        let (
            Some(wireframe_pipeline),
            Some(wireframe_buffer),
            Some(wireframe_mesh),
            Some(view_bind_group),
            Some(depth_texture),
            Some(enironent),
        ) = (
            world.get_resource::<WireframePipeline>(),
            world.get_resource::<WireframeObjectBuffer>(),
            world.get_resource::<UnitCubeMesh>(),
            world.get_resource::<CentralCameraViewUniform>(),
            world.get_resource::<MainDepthTextureResource>(),
            world.get_resource::<EnvironmentUniforms>(),
        )
        else {
            warn!("Missing one or more required resources for the Wireframe Pass. Skipping pass.");
            return;
        };

        if wireframe_buffer.objects.is_empty() {
            return;
        }

        // INFO: ----------------------
        //         do rendering
        // ----------------------------

        let mut render_pass =
            render_context
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Wireframe Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_context.surface_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

        // do render
        render_pass.set_pipeline(wireframe_pipeline);

        render_pass.set_bind_group(0, &view_bind_group.bind_group, &[]);
        render_pass.set_bind_group(1, &enironent.bind_group, &[]);
        render_pass.set_bind_group(2, &wireframe_buffer.bind_group, &[]);

        render_pass.set_vertex_buffer(0, wireframe_mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            wireframe_mesh.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(
            0..wireframe_mesh.index_count,
            0,
            0..wireframe_buffer.objects.len() as u32,
        );
    }
}
