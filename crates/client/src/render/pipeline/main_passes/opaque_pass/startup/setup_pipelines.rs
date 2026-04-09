use crate::input::systems::toggle_opaque_wireframe::OpaqueWireframeMode;
use crate::render::pipeline::{
    gpu_resources::world_uniforms::ChunkStorageBindGroupLayout,
    main_passes::shared_resources::{
        CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout, MAIN_DEPTH_FORMAT,
        TextureArrayBindGroupLayout,
    },
    shader_registry::{
        OPAQUE_FRAG_SHADER_HANDLE, OPAQUE_VERT_SHADER_HANDLE, SKYBOX_FRAG_SHADER_HANDLE,
        SKYBOX_VERT_SHADER_HANDLE,
    },
};
use bevy::{
    ecs::prelude::*,
    render::{extract_resource::ExtractResource, render_resource::*, view::ViewTarget},
};
use tracing::instrument;

/// A resource that defines the current opaque render mode
#[derive(Resource, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

/// A key that uniquely identifies a specialized opaque pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaquePipelineKey {
    pub msaa_samples: u32,
    pub hdr: bool,
    pub mode: OpaqueRenderMode,
    pub is_skybox: bool,
}

/// A resource that holds all the layouts and handles needed to specialize opaque pipelines.
#[derive(Resource)]
pub struct OpaquePipelines {
    pub view_layout: BindGroupLayoutDescriptor,
    pub environment_layout: BindGroupLayoutDescriptor,
    pub texture_layout: BindGroupLayoutDescriptor,
    pub chunk_storage_layout: BindGroupLayoutDescriptor,
}

impl FromWorld for OpaquePipelines {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let texture_layout = world.resource::<TextureArrayBindGroupLayout>();
        let chunk_storage_layout = world.resource::<ChunkStorageBindGroupLayout>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
            texture_layout: texture_layout.descriptor.clone(),
            chunk_storage_layout: chunk_storage_layout.descriptor.clone(),
        }
    }
}

impl SpecializedRenderPipeline for OpaquePipelines {
    type Key = OpaquePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let fragment_target = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        if key.is_skybox {
            let depth_stencil = Some(DepthStencilState {
                format: MAIN_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            });

            RenderPipelineDescriptor {
                label: Some("Skybox Opaque Pipeline".into()),
                layout: vec![self.view_layout.clone(), self.environment_layout.clone()],
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
                    targets: fragment_target.to_vec(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil,
                multisample: MultisampleState {
                    count: key.msaa_samples,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                zero_initialize_workgroup_memory: true,
            }
        } else {
            let depth_stencil = Some(DepthStencilState {
                format: MAIN_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            });

            let polygon_mode = match key.mode {
                OpaqueRenderMode::Fill => PolygonMode::Fill,
                OpaqueRenderMode::Wireframe => PolygonMode::Line,
            };

            let label = match key.mode {
                OpaqueRenderMode::Fill => "Opaque Pipeline",
                OpaqueRenderMode::Wireframe => "Wireframe Opaque Pipeline",
            };

            RenderPipelineDescriptor {
                label: Some(label.into()),
                layout: vec![
                    self.view_layout.clone(),
                    self.environment_layout.clone(),
                    self.texture_layout.clone(),
                    self.chunk_storage_layout.clone(),
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
                    targets: fragment_target.to_vec(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    polygon_mode,
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                depth_stencil,
                multisample: MultisampleState {
                    count: key.msaa_samples,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                zero_initialize_workgroup_memory: true,
            }
        }
    }
}
