use crate::{
    prelude::*,
    render::{
        pipeline::main_passes::bounding_box_pass::{
            extract::WireframeToggleState,
            gpu_resources::{
                WireframeObjectBuffer, WireframeObjectData,
                object_binding::WireframeObjectBindGroupLayout,
            },
        },
        types::RenderTransformComponent,
    },
};
use bevy::{
    ecs::prelude::*,
    render::{
        render_resource::{BindGroupEntry, BufferDescriptor, BufferUsages},
        renderer::{RenderDevice, RenderQueue},
    },
};
use shared::simulation::{
    block::TargetedBlock,
    chunk::consts::{CHUNK_DEPTH, CHUNK_HEIGHT, CHUNK_WIDTH},
};

#[instrument(skip_all)]
pub fn queue_wireframe_system(
    // input
    queue: Res<RenderQueue>,
    device: Res<RenderDevice>,

    object_layout: Res<WireframeObjectBindGroupLayout>,

    chunk_query: Query<&RenderTransformComponent>,
    active_bounds: Res<WireframeToggleState>,
    targeted_block: Res<TargetedBlock>,

    // output
    mut wireframe_buffer: ResMut<WireframeObjectBuffer>,
) {
    wireframe_buffer.objects.clear();

    if !active_bounds.enabled && targeted_block.position.is_none() {
        return;
    }

    if active_bounds.enabled {
        // add all chunk wireframes
        let translation_matrix = Mat4::from_translation(vec3(
            CHUNK_WIDTH as f32 / 2.0,
            CHUNK_HEIGHT as f32 / 2.0,
            CHUNK_DEPTH as f32 / 2.0,
        ));
        let scale_matrix = Mat4::from_scale(vec3(
            CHUNK_WIDTH as f32,
            CHUNK_HEIGHT as f32,
            CHUNK_DEPTH as f32,
        ));

        for transform in chunk_query.iter() {
            let model_matrix = transform.transform * translation_matrix * scale_matrix;

            wireframe_buffer.objects.push(WireframeObjectData {
                model_matrix: model_matrix.to_cols_array(),
            });
        }
    }

    // add targeted block wireframe if exists
    if let Some(block_pos) = targeted_block.position {
        let block_translation = vec3(
            block_pos.x as f32 + 0.5,
            block_pos.y as f32 + 0.5,
            block_pos.z as f32 + 0.5,
        );
        let block_translation_matrix = Mat4::from_translation(block_translation);
        let model_matrix = Mat4::IDENTITY * block_translation_matrix;

        wireframe_buffer.objects.push(WireframeObjectData {
            model_matrix: model_matrix.to_cols_array(),
        });
    }

    if !wireframe_buffer.objects.is_empty() {
        let buffer_size =
            (wireframe_buffer.objects.len() * std::mem::size_of::<WireframeObjectData>()) as u64;

        if wireframe_buffer.buffer.size() < buffer_size {
            let new_size = (buffer_size as f64 * 1.5).ceil() as u64;

            debug!(
                target : "gpu_memory",
                "Resizing wireframe object buffer to {} KB",
                new_size / 1024
            );

            let new_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Wireframe Object Buffer"),
                size: new_size,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            wireframe_buffer.bind_group = device.create_bind_group(
                Some("Wireframe Object Bind Group"),
                &object_layout.layout,
                &[BindGroupEntry {
                    binding: 0,
                    resource: new_buffer.as_entire_binding(),
                }],
            );

            wireframe_buffer.buffer = new_buffer;
        }

        // write data to the buffer (which might be new/resized)
        queue.write_buffer(
            &wireframe_buffer.buffer,
            0,
            bytemuck::cast_slice(&wireframe_buffer.objects),
        );
    }
}
