use crate::render::pipeline::gpu_resources::world_uniforms::ChunkStorageBindGroupLayout;
use crate::render::pipeline::main_passes::shared_resources::main_depth_texture::MAIN_DEPTH_FORMAT;
use crate::render::pipeline::main_passes::shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout, TextureArrayBindGroupLayout,
};
use crate::render::pipeline::shader_registry::{
    OPAQUE_FRAG_SHADER_HANDLE, OPAQUE_VERT_SHADER_HANDLE, SKYBOX_FRAG_SHADER_HANDLE,
    SKYBOX_VERT_SHADER_HANDLE,
};
use crate::simulation::input::systems::toggle_opaque_wireframe::OpaqueWireframeMode;
use bevy::ecs::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::*;
use tracing::instrument;

/// A resource that defines the current opaque render mode
#[derive(Resource, Default, Debug, PartialEq, Clone, Copy)]
pub enum OpaqueRenderMode {
    #[default]
    Fill,
    Wireframe,
}

impl ExtractResource for OpaqueRenderMode {
    type Source = OpaqueWireframeMode;

    fn extract_resource(source: &Self::Source) -> Self {
        if source.enabled {
            OpaqueRenderMode::Wireframe
        } else {
            OpaqueRenderMode::Fill
        }
    }
}

/// A resource that holds all the opaque phase pipelines.
#[derive(Resource)]
pub struct OpaquePipelines {
    /// A pipeline that draws filled opaque geometry.
    pub fill_id: CachedRenderPipelineId,

    /// A pipeline that draws wireframe opaque geometry.
    pub wireframe_id: CachedRenderPipelineId,

    /// A pipeline that draws the skybox.
    pub skybox_id: CachedRenderPipelineId,
}

impl FromWorld for OpaquePipelines {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let texture_layout = world.resource::<TextureArrayBindGroupLayout>();
        let chunk_storage_layout = world.resource::<ChunkStorageBindGroupLayout>();

        let opaque_fragment_target = [Some(ColorTargetState {
            format: TextureFormat::Rgba8UnormSrgb,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let opaque_depth_stencil = Some(DepthStencilState {
            format: MAIN_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        // INFO: ---------------------------------
        //         regular opaque pipeline
        // ---------------------------------------

        let fill_pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Opaque Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                environment_layout.descriptor.clone(),
                texture_layout.descriptor.clone(),
                chunk_storage_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: OPAQUE_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: OPAQUE_FRAG_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: opaque_fragment_target.to_vec(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                polygon_mode: PolygonMode::Fill,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: opaque_depth_stencil.clone(),
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        // INFO: -----------------------------------
        //         wireframe opaque pipeline
        // -----------------------------------------

        let wireframe_pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Wireframe Opaque Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                environment_layout.descriptor.clone(),
                texture_layout.descriptor.clone(),
                chunk_storage_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: OPAQUE_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: OPAQUE_FRAG_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: opaque_fragment_target.to_vec(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                polygon_mode: PolygonMode::Line,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: opaque_depth_stencil,
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        // INFO: --------------------------------
        //         skybox opaque pipeline
        // --------------------------------------

        let skybox_depth_stencil = Some(DepthStencilState {
            format: MAIN_DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        let skybox_pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Skybox Opaque Pipeline".into()),
            layout: vec![
                view_layout.descriptor.clone(),
                environment_layout.descriptor.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: SKYBOX_VERT_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: SKYBOX_FRAG_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fs_main".into()),
                targets: opaque_fragment_target.to_vec(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                polygon_mode: PolygonMode::Fill,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: skybox_depth_stencil,
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: true,
        };

        let pipeline_cache = world.resource_mut::<PipelineCache>();

        Self {
            fill_id: pipeline_cache.queue_render_pipeline(fill_pipeline_descriptor),
            wireframe_id: pipeline_cache.queue_render_pipeline(wireframe_pipeline_descriptor),
            skybox_id: pipeline_cache.queue_render_pipeline(skybox_pipeline_descriptor),
        }
    }
}
