use crate::prelude::*;
use crate::render::global_extract::resources::RenderCameraResource;
use crate::render::pipeline::shadow_pass::gpu_resources::{
    ShadowDepthTextureResource, ShadowViewBuffer,
};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindingResource, Buffer, BufferDescriptor, BufferUsages,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bytemuck::{Pod, Zeroable};
use shared::simulation::chunk::{CHUNK_SIDE_LENGTH, RENDER_DISTANCE};

// INFO: ---------------------------------
//         uniform data definition
// ---------------------------------------

/// The data structure representing the camera view information to be stored in the view buffer.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct CentralCameraViewData {
    /// combined view-projection matrix
    pub view_proj_matrix: [f32; 16],

    /// inverse of the combined view-projection matrix
    pub inverse_view_proj_matrix: [f32; 16],

    /// position of camera for distance-based fog
    pub world_position: [f32; 3],

    /// Render distance for fog calculations
    pub render_distance: f32,
}

// INFO: -------------------------------
//         GPU uniform resources
// -------------------------------------

/// The view bind group layout resource shared by all camera-centric render passes.
#[derive(Resource)]
pub struct CentralCameraViewBindGroupLayout {
    pub layout: bevy::render::render_resource::BindGroupLayout,
    pub descriptor: bevy::render::render_resource::BindGroupLayoutDescriptor,
}

impl FromWorld for CentralCameraViewBindGroupLayout {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();

        let entries = vec![
            // main view buffer
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 0,
                visibility: bevy::render::render_resource::ShaderStages::VERTEX
                    | bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Buffer {
                    ty: bevy::render::render_resource::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // shadow view buffer
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 1,
                visibility: bevy::render::render_resource::ShaderStages::VERTEX
                    | bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Buffer {
                    ty: bevy::render::render_resource::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // shadow texture map
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 2,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Texture {
                    sample_type: bevy::render::render_resource::TextureSampleType::Depth,
                    view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // shadow texture sampler
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: 3,
                visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                ty: bevy::render::render_resource::BindingType::Sampler(
                    bevy::render::render_resource::SamplerBindingType::Comparison,
                ),
                count: None,
            },
        ];

        let descriptor = bevy::render::render_resource::BindGroupLayoutDescriptor {
            label: "Central Camera View Bind Group Layout".into(),
            entries,
        };

        let layout =
            device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

/// A GPU buffer resource containing the shared camera view data for all camera-based passes.
#[derive(Resource)]
pub struct CentralCameraViewUniform {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
}

impl FromWorld for CentralCameraViewUniform {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();
        let layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let shadow_view_buffer = world.resource::<ShadowViewBuffer>();
        let shadow_map = world.resource::<ShadowDepthTextureResource>();

        let view_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Central Camera View Buffer"),
            size: std::mem::size_of::<CentralCameraViewData>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_bind_group = device.create_bind_group(
            Some("Central Camera View Bind Group"),
            &layout.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: shadow_view_buffer.buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&shadow_map.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&shadow_map.sampler),
                },
            ],
        );

        Self {
            buffer: view_buffer,
            bind_group: view_bind_group,
        }
    }
}

// INFO: ----------------------------
//         Management systems
// ----------------------------------

/// Takes the extracted camera data and uploads it to the GPU buffer for any pass that needs it.
///
/// Since the camera is essentially constantly rechanging this needs to be run just about every
/// frame. I am not sure it is worth adding the optimization to only run on camera updates as
/// the write buffer here is pretty cheap.
#[instrument(skip_all)]
pub fn update_camera_view_buffer_system(
    // Input
    camera_info: Res<RenderCameraResource>,
    view_buffer: Res<CentralCameraViewUniform>,

    // Output (writing buffer to queue)
    queue: Res<RenderQueue>,
) {
    let view_proj_matrix = camera_info.projection_matrix * camera_info.view_matrix;

    let camera_data = CentralCameraViewData {
        view_proj_matrix: view_proj_matrix.to_cols_array(),
        inverse_view_proj_matrix: view_proj_matrix.inverse().to_cols_array(),
        world_position: camera_info.world_position.into(),
        render_distance: (RENDER_DISTANCE * CHUNK_SIDE_LENGTH as i32) as f32,
    };

    queue.write_buffer(&view_buffer.buffer, 0, bytemuck::cast_slice(&[camera_data]));
}
