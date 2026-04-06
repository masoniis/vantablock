use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, Buffer, BufferDescriptor, BufferUsages,
};
use bevy::render::renderer::RenderDevice;
use bytemuck::{Pod, Zeroable};

// INFO: ----------------------------
//         uniform definition
// ----------------------------------

/// The per-object data (model matrix) for a single wireframe instance.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct WireframeObjectData {
    pub model_matrix: [f32; 16],
}

// INFO: -----------------------------------------
//         GPU binding, buffer, and layout
// -----------------------------------------------

#[derive(Resource)]
pub struct WireframeObjectBindGroupLayout {
    pub layout: bevy::render::render_resource::BindGroupLayout,
    pub descriptor: bevy::render::render_resource::BindGroupLayoutDescriptor,
}

impl FromWorld for WireframeObjectBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let descriptor = bevy::render::render_resource::BindGroupLayoutDescriptor {
            label: "Wireframe Object Layout".into(),
            entries: vec![
                // object storage buffer (matrices as seen above)
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: bevy::render::render_resource::ShaderStages::VERTEX,
                    ty: bevy::render::render_resource::BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        };

        let layout =
            device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

/// A resource holding the GPU buffer and bind group for wireframe object data.
#[derive(Resource)]
pub struct WireframeObjectBuffer {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub objects: Vec<WireframeObjectData>,
}

impl FromWorld for WireframeObjectBuffer {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = world.resource::<WireframeObjectBindGroupLayout>();

        let initial_capacity = 128;
        let object_buffer_size =
            (initial_capacity as u64) * std::mem::size_of::<WireframeObjectData>() as u64;

        let object_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Wireframe Object Buffer"),
            size: object_buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let object_bind_group = device.create_bind_group(
            Some("Wireframe Object Bind Group"),
            &layout.layout,
            &[BindGroupEntry {
                binding: 0,
                resource: object_buffer.as_entire_binding(),
            }],
        );

        Self {
            buffer: object_buffer,
            bind_group: object_bind_group,
            objects: Vec::with_capacity(initial_capacity),
        }
    }
}
