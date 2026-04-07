use super::super::shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout,
};
use crate::render::pipeline::gpu_resources::world_uniforms::ChunkStorageBindGroupLayout;
use crate::render::pipeline::main_passes::shared_resources::TextureArrayBindGroupLayout;
use crate::render::pipeline::main_passes::shared_resources::main_depth_texture::MAIN_DEPTH_FORMAT;
use crate::render::pipeline::shader_registry::{
    TRANSPARENT_FRAG_SHADER_HANDLE, TRANSPARENT_VERT_SHADER_HANDLE,
};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::*;

// INFO: -------------------
//         resources
// -------------------------

#[derive(Resource)]
pub struct TransparentPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for TransparentPipeline {
    fn from_world(world: &mut World) -> Self {
        // layouts
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let texture_layout = world.resource::<TextureArrayBindGroupLayout>();
        let chunk_storage_layout = world.resource::<ChunkStorageBindGroupLayout>();

        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Transparent Render Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                environment_layout.descriptor.clone(),
                texture_layout.descriptor.clone(),
                chunk_storage_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: TRANSPARENT_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: TRANSPARENT_FRAG_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: MAIN_DEPTH_FORMAT,
                depth_write_enabled: false, // transparent objects don't write depth
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            },
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        let pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_render_pipeline(pipeline_descriptor);

        Self { pipeline_id }
    }
}
