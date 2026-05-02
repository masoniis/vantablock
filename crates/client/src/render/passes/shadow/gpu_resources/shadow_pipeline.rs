use super::shadow_view_uniform::ShadowViewBindGroupLayout;
use crate::render::passes::shadow::gpu_resources::SHADOW_DEPTH_FORMAT;
use crate::render::resources::world_uniforms::ChunkStorageBindGroupLayout;
use crate::render::shaders::SHADOW_VERT_SHADER_HANDLE;
use bevy::ecs::prelude::*;
use bevy::mesh::PrimitiveTopology;
use bevy::render::render_resource::{
    CachedRenderPipelineId, CompareFunction, DepthBiasState, DepthStencilState, Face, FrontFace,
    MultisampleState, PipelineCache, PolygonMode, PrimitiveState, RenderPipelineDescriptor,
    StencilState, VertexState,
};

/// A resource that holds the shadow pass pipeline.
#[derive(Resource)]
pub struct ShadowPassPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for ShadowPassPipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<ShadowViewBindGroupLayout>();
        let chunk_layout = world.resource::<ChunkStorageBindGroupLayout>();

        let shadow_depth_stencil = Some(DepthStencilState {
            format: SHADOW_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Shadow Pass Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                chunk_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: SHADOW_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                polygon_mode: PolygonMode::Fill,
                cull_mode: Some(Face::Front),
                ..Default::default()
            },
            depth_stencil: shadow_depth_stencil,
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        let pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline_id = pipeline_cache.queue_render_pipeline(pipeline_descriptor);

        Self { pipeline_id }
    }
}
