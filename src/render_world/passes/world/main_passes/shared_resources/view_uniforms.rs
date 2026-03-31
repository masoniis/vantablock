use crate::prelude::*;
use crate::render_world::passes::world::shadow_pass::gpu_resources::{
    ShadowDepthTextureResource, ShadowViewBuffer,
};
use crate::render_world::{
    global_extract::resources::RenderCameraResource,
    graphics_context::resources::{RenderDevice, RenderQueue},
};
use crate::simulation_world::chunk::{CHUNK_SIDE_LENGTH, RENDER_DISTANCE};
use bevy::ecs::prelude::*;
use bytemuck::{Pod, Zeroable};

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
pub struct CentralCameraViewBindGroupLayout(pub wgpu::BindGroupLayout);

impl FromWorld for CentralCameraViewBindGroupLayout {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Central Camera View Bind Group Layout"),
            entries: &[
                // main view buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // shadow view buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // shadow texture map
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // shadow texture sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        Self(layout)
    }
}

/// A GPU buffer resource containing the shared camera view data for all camera-based passes.
#[derive(Resource)]
pub struct CentralCameraViewUniform {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl FromWorld for CentralCameraViewUniform {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>().clone();
        let layout = world.resource::<CentralCameraViewBindGroupLayout>();
        let shadow_view_buffer = world.resource::<ShadowViewBuffer>();
        let shadow_map = world.resource::<ShadowDepthTextureResource>();

        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Central Camera View Buffer"),
            size: std::mem::size_of::<CentralCameraViewData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Central Camera View Bind Group"),
            layout: &layout.0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: shadow_view_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                },
            ],
        });

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
