use crate::render::passes::bounding_box::WireframeObjectBindGroupLayout;
use crate::render::passes::bounding_box::gpu_resources::WireframeVertex;
use crate::render::{
    environment::EnvironmentBindGroupLayout,
    shaders::{WorldShader, WorldShaders},
    view::CentralCameraViewBindGroupLayout,
};
use bevy::asset::Handle;
use bevy::core_pipeline::core_3d;
use bevy::ecs::prelude::*;
use bevy::render::render_resource::*;
use bevy::shader::Shader;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WireframePipelineKey {
    pub msaa_samples: u32,
    pub hdr: bool,
}

/// A resource holding the layouts and handles for debug wireframes.
#[derive(Resource)]
pub struct WireframePipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub environment_layout: BindGroupLayoutDescriptor,
    pub object_layout: BindGroupLayoutDescriptor,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
}

impl FromWorld for WireframePipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let object_layout = world.resource::<WireframeObjectBindGroupLayout>();
        let shaders = world.resource::<WorldShaders>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
            object_layout: object_layout.descriptor.clone(),
            vertex_shader: shaders.get(WorldShader::WireframeVert),
            fragment_shader: shaders.get(WorldShader::WireframeFrag),
        }
    }
}

impl SpecializedRenderPipeline for WireframePipeline {
    type Key = WireframePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                self.environment_layout.clone(),
                self.object_layout.clone(),
            ],
            immediate_size: 0,
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                shader_defs: vec![],
                constants: vec![],
                entry_point: Some("vs_main".into()),
                buffers: vec![WireframeVertex::desc()],
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec![],
                constants: vec![],
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: core_3d::CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
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
            multisample: MultisampleState {
                count: key.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: true,
        }
    }
}
