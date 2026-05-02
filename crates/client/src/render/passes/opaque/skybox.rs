use crate::render::resources::{CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout};
use crate::render::shaders::{SKYBOX_FRAG_SHADER_HANDLE, SKYBOX_VERT_SHADER_HANDLE};
use bevy::{
    core_pipeline::core_3d,
    ecs::prelude::*,
    render::{render_resource::*, view::ViewTarget},
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
}

impl FromWorld for OpaqueSkyboxPipeline {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
        }
    }
}

impl SpecializedRenderPipeline for OpaqueSkyboxPipeline {
    type Key = OpaqueSkyboxPipelineKey;

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

        let depth_stencil = Some(DepthStencilState {
            format: core_3d::CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Equal,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });

        RenderPipelineDescriptor {
            label: Some("Skybox Pipeline".into()),
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
    }
}
