use crate::render::pipeline::main_passes::bounding_box_pass::WireframeObjectBindGroupLayout;
use crate::render::pipeline::main_passes::shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout, MAIN_DEPTH_FORMAT,
};
use crate::render::pipeline::shader_registry::{
    WIREFRAME_FRAG_SHADER_HANDLE, WIREFRAME_VERT_SHADER_HANDLE,
};
use crate::render::types::WireframeVertex;
use bevy::ecs::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::view::ViewTarget;

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
}

impl FromWorld for WireframePipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let environment_layout = world.resource::<EnvironmentBindGroupLayout>();
        let object_layout = world.resource::<WireframeObjectBindGroupLayout>();

        Self {
            view_layout: view_layout.descriptor.clone(),
            environment_layout: environment_layout.descriptor.clone(),
            object_layout: object_layout.descriptor.clone(),
        }
    }
}

impl SpecializedRenderPipeline for WireframePipeline {
    type Key = WireframePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
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
                    format,
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
            multisample: MultisampleState {
                count: key.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: true,
        }
    }
}
