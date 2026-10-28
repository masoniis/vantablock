use crate::{
    debug::wireframe::OpaqueRenderMode,
    prelude::*,
    render::{
        chunk::ChunkStorageBindGroupLayout,
        environment::EnvironmentBindGroupLayout,
        shaders::{WorldShader, WorldShaders},
        texture::TextureArrayBindGroupLayout,
        view::CentralCameraViewBindGroupLayout,
    },
};
use bevy::{
    asset::Handle,
    core_pipeline::core_3d::{self},
    ecs::prelude::*,
    render::render_resource::*,
    shader::Shader,
};

/// A key that uniquely identifies a specialized world opaque pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opaque3dPipelineKey {
    pub msaa_samples: u32,
    pub hdr: bool,
    pub mode: OpaqueRenderMode,
}

/// A resource that holds all the layouts and handles needed to specialize world opaque pipelines.
#[derive(Resource)]
pub struct Opaque3dPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub environment_layout: BindGroupLayoutDescriptor,
    pub texture_layout: BindGroupLayoutDescriptor,
    pub chunk_storage_layout: BindGroupLayoutDescriptor,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
}

impl FromWorld for Opaque3dPipeline {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let texture_layout = world.resource::<TextureArrayBindGroupLayout>();
        let chunk_storage_layout = world.resource::<ChunkStorageBindGroupLayout>();
        let shaders = world.resource::<WorldShaders>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
            texture_layout: texture_layout.descriptor.clone(),
            chunk_storage_layout: chunk_storage_layout.descriptor.clone(),
            vertex_shader: shaders.get(WorldShader::OpaqueVert),
            fragment_shader: shaders.get(WorldShader::OpaqueFrag),
        }
    }
}

impl SpecializedRenderPipeline for Opaque3dPipeline {
    type Key = Opaque3dPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        let fragment_target = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let depth_stencil = Some(DepthStencilState {
            format: core_3d::CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        let polygon_mode = match key.mode {
            OpaqueRenderMode::Fill => PolygonMode::Fill,
            OpaqueRenderMode::Wireframe => PolygonMode::Line,
        };

        let label = match key.mode {
            OpaqueRenderMode::Fill => "World Opaque Pipeline",
            OpaqueRenderMode::Wireframe => "Wireframe World Opaque Pipeline",
        };

        RenderPipelineDescriptor {
            label: Some(label.into()),
            layout: vec![
                self.view_layout.clone(),
                self.environment_layout.clone(),
                self.texture_layout.clone(),
                self.chunk_storage_layout.clone(),
            ],
            immediate_size: 0,
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                shader_defs: vec![],
                constants: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec![],
                constants: vec![],
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
