use crate::render::types::WireframeVertex;
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{Buffer, BufferInitDescriptor, BufferUsages};
use bevy::render::renderer::RenderDevice;

/// A 1x1x1 wireframe cube mesh to be used for representing chunk bounding boxes.
#[derive(Resource)]
pub struct UnitCubeMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl FromWorld for UnitCubeMesh {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let dummy_color = [1.0, 1.0, 1.0];

        #[rustfmt::skip]
        let vertices: [WireframeVertex; 8] = [
            // bottom face
            WireframeVertex::new([-0.5, -0.5, -0.5], dummy_color),
            WireframeVertex::new([0.5, -0.5, -0.5], dummy_color),
            WireframeVertex::new([0.5, -0.5, 0.5], dummy_color),
            WireframeVertex::new([-0.5, -0.5, 0.5], dummy_color),
            // top face
            WireframeVertex::new([-0.5, 0.5, -0.5], dummy_color),
            WireframeVertex::new([0.5, 0.5, -0.5], dummy_color),
            WireframeVertex::new([0.5, 0.5, 0.5], dummy_color),
            WireframeVertex::new([-0.5, 0.5, 0.5],dummy_color),
        ];

        // buffer
        let vertex_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Debug Wireframe Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        // 12 lines, 2 indices per line
        #[rustfmt::skip]
        let indices: [u32; 24] = [
            0, 1, 1, 2, 2, 3, 3, 0, // bottom square
            4, 5, 5, 6, 6, 7, 7, 4, // top square
            0, 4, 1, 5, 2, 6, 3, 7, // vertical connectors
        ];

        let index_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Debug Wireframe Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}
