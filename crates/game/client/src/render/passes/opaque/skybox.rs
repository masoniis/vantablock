use crate::render::{
    environment::EnvironmentBindGroupLayout,
    shaders::{WorldShader, WorldShaders},
    view::CentralCameraViewBindGroupLayout,
};
use bevy::{
    asset::Handle, core_pipeline::core_3d, ecs::prelude::*, render::render_resource::*,
    shader::Shader,
};
use tracing::instrument;

/// A key that uniquely identifies a specialized skybox pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaqueSkyboxPipelineKey {
    pub msaa_samples: u32,
    pub hdr: bool,
}

/// A resource that holds all the layouts and handles needed to specialize skybox pipelines.
#[derive(Resource)]
pub struct OpaqueSkyboxPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub environment_layout: BindGroupLayoutDescriptor,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
}

impl FromWorld for OpaqueSkyboxPipeline {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let shaders = world.resource::<WorldShaders>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
            vertex_shader: shaders.get(WorldShader::SkyboxVert),
            fragment_shader: shaders.get(WorldShader::SkyboxFrag),
        }
    }
}

impl SpecializedRenderPipeline for OpaqueSkyboxPipeline {
    type Key = OpaqueSkyboxPipelineKey;

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
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::Equal),
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        RenderPipelineDescriptor {
            label: Some("Skybox Pipeline".into()),
            layout: vec![self.view_layout.clone(), self.environment_layout.clone()],
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
    }
}
