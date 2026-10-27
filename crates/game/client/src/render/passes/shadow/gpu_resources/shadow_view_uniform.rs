use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, Buffer, BufferDescriptor, BufferUsages,
};
use bevy::render::renderer::RenderDevice;
use bytemuck::{Pod, Zeroable};

// INFO: ----------------------------
//         uniform definition
// ----------------------------------

/// The shadow "camera" (i.e., the sun's) view uniform.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct ShadowViewData {
    pub light_view_proj_matrix: [f32; 16],
}

// INFO: -----------------------------------------
//         GPU binding, buffer, and layout
// -----------------------------------------------

/// The shadow pass "sun camera" bind group layout.
#[derive(Resource)]
pub struct ShadowViewBindGroupLayout {
    pub layout: bevy::render::render_resource::BindGroupLayout,
    pub descriptor: bevy::render::render_resource::BindGroupLayoutDescriptor,
}

impl FromWorld for ShadowViewBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let descriptor = bevy::render::render_resource::BindGroupLayoutDescriptor {
            label: "Shadow View Bind Group Layout".into(),
            entries: vec![
                // slot for `ShadowViewData` uniform defined above
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: bevy::render::render_resource::ShaderStages::VERTEX
                        | bevy::render::render_resource::ShaderStages::FRAGMENT,
                    ty: bevy::render::render_resource::BindingType::Buffer {
                        ty: bevy::render::render_resource::BufferBindingType::Uniform,
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

/// A GPU buffer resource containing the shadow pass's view data.
#[derive(Resource)]
pub struct ShadowViewBuffer {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
}

impl FromWorld for ShadowViewBuffer {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let view_layout = world.resource::<ShadowViewBindGroupLayout>();

        let view_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Shadow View Buffer"),
            size: std::mem::size_of::<ShadowViewData>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_bind_group = device.create_bind_group(
            Some("Shadow View Bind Group"),
            &view_layout.layout,
            &[BindGroupEntry {
                binding: 0,
                resource: view_buffer.as_entire_binding(),
            }],
        );

        Self {
            buffer: view_buffer,
            bind_group: view_bind_group,
        }
    }
}
