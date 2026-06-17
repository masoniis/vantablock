use crate::prelude::*;
use crate::render::data::{ExtractedSun, RenderTimeResource};
use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, Buffer, BufferDescriptor, BufferUsages,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bytemuck::{Pod, Zeroable};

// INFO: ----------------------------
//         uniform definition
// ----------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct EnvironmentData {
    pub main_light_direction: [f32; 3],
    pub time: f32,

    pub main_light_color: [f32; 3],
    pub ambient_strength: f32,

    pub sun_direction: [f32; 3],
    pub _padding1: u32,

    pub sun_disk_color: [f32; 3],
    pub _padding2: u32,

    pub moon_direction: [f32; 3],
    pub _padding3: u32,

    pub horizon_color: [f32; 3],
    pub _padding4: u32,

    pub zenith_color: [f32; 3],
    pub _padding5: u32,
}

impl EnvironmentData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        main_light_direction: [f32; 3],
        time: f32,
        main_light_color: [f32; 3],
        ambient_strength: f32,
        sun_direction: [f32; 3],
        sun_disk_color: [f32; 3],
        moon_direction: [f32; 3],
        horizon_color: [f32; 3],
        zenith_color: [f32; 3],
    ) -> Self {
        Self {
            main_light_direction,
            time,

            main_light_color,
            ambient_strength,

            sun_direction,
            _padding1: 0,

            sun_disk_color,
            _padding2: 0,

            moon_direction,
            _padding3: 0,

            horizon_color,
            _padding4: 0,

            zenith_color,
            _padding5: 0,
        }
    }
}

// INFO: -----------------------------------------
//         GPU binding, buffer, and layout
// -----------------------------------------------

/// The environment bind group layout resource shared by all camera-centric render passes.
#[derive(Resource)]
pub struct EnvironmentBindGroupLayout {
    pub layout: bevy::render::render_resource::BindGroupLayout,
    pub descriptor: bevy::render::render_resource::BindGroupLayoutDescriptor,
}

impl FromWorld for EnvironmentBindGroupLayout {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let descriptor = bevy::render::render_resource::BindGroupLayoutDescriptor {
            label: "Environment Bind Group Layout".into(),
            entries: vec![
                // holds buffer for `EnvironmentData`
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
            ],
        };

        let layout =
            device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

/// A GPU buffer resource containing the shared environment data for all central-camera-based passes.
#[derive(Resource)]
pub struct EnvironmentUniforms {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
}

impl FromWorld for EnvironmentUniforms {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = world.resource::<EnvironmentBindGroupLayout>();

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Environment Buffer"),
            size: std::mem::size_of::<EnvironmentData>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(
            Some("Environment Bind Group"),
            &layout.layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );

        Self { buffer, bind_group }
    }
}

// INFO: -----------------------
//         update system
// -----------------------------

/// A system to prepare the environment buffer data for centric passes.
#[instrument(skip_all)]
pub fn update_environment_uniform_buffer_system(
    // Input (target buffer)
    buffer: Res<EnvironmentUniforms>,
    extracted_sun: Res<ExtractedSun>,
    extracted_time: Res<RenderTimeResource>,

    // Output (writing buffer to queue)
    queue: Res<RenderQueue>,
) {
    let environtment_data = EnvironmentData::new(
        extracted_sun.main_light_direction,
        extracted_time.total_elapsed_seconds,
        extracted_sun.main_light_color,
        extracted_sun.ambient_strength,
        extracted_sun.sun_direction,
        extracted_sun.sun_disk_color,
        extracted_sun.moon_direction,
        extracted_sun.horizon,
        extracted_sun.zenith,
    );

    queue.write_buffer(
        &buffer.buffer,
        0,
        bytemuck::cast_slice(&[environtment_data]),
    );
}
