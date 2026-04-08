use super::super::shared_resources::{
    CentralCameraViewBindGroupLayout, EnvironmentBindGroupLayout, TextureArrayBindGroupLayout,
    MAIN_DEPTH_FORMAT,
};
use crate::render::pipeline::{
    gpu_resources::world_uniforms::ChunkStorageBindGroupLayout,
    shader_registry::{TRANSPARENT_FRAG_SHADER_HANDLE, TRANSPARENT_VERT_SHADER_HANDLE},
};
use bevy::prelude::{FromWorld, Resource, World};
use bevy::{render::render_resource::*, render::view::ViewTarget};

// INFO: -------------------
//         resources
// -------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransparentPipelineKey {
    pub msaa_samples: u32,
    pub hdr: bool,
}

// TODO: we should storethe actual layouts, NOT the descriptors
#[derive(Resource)]
pub struct TransparentPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub environment_layout: BindGroupLayoutDescriptor,
    pub texture_layout: BindGroupLayoutDescriptor,
    pub chunk_storage_layout: BindGroupLayoutDescriptor,
}

impl FromWorld for TransparentPipeline {
    fn from_world(world: &mut World) -> Self {
        // layouts
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

impl SpecializedRenderPipeline for TransparentPipeline {
    type Key = TransparentPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        RenderPipelineDescriptor {
            label: Some("Transparent Render Pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                self.environment_layout.clone(),
                self.texture_layout.clone(),
                self.chunk_storage_layout.clone(),
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
                    format,
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
            multisample: MultisampleState {
                count: key.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: true,
        }
    }
}
