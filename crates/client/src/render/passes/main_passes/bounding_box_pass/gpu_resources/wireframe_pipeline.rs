use crate::render::passes::main_passes::bounding_box_pass::WireframeObjectBindGroupLayout;
use crate::render::passes::main_passes::shared_resources::main_depth_texture::MAIN_DEPTH_FORMAT;
use crate::render::passes::main_passes::shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout,
};
use crate::render::passes::shader_registry::{
    WIREFRAME_FRAG_SHADER_HANDLE, WIREFRAME_VERT_SHADER_HANDLE,
};
use crate::render::types::WireframeVertex;
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
    DepthBiasState, DepthStencilState, FragmentState, FrontFace, MultisampleState, PipelineCache,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, StencilState,
    VertexState,
};

/// A resource holding the pipeline for rendering debug wireframes.
#[derive(Resource)]
pub struct WireframePipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for WireframePipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let object_layout = world.resource::<WireframeObjectBindGroupLayout>();

        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                environment_layout.descriptor.clone(),
                object_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: WIREFRAME_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![WireframeVertex::desc()],
            },
            fragment: Some(FragmentState {
                shader: WIREFRAME_FRAG_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: MAIN_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Line,
                ..Default::default()
            },
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            pipeline_id: pipeline_cache.queue_render_pipeline(pipeline_descriptor),
        }
    }
}
