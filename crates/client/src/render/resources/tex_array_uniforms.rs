use crate::prelude::*;
use crate::render::texture::BlockTextureArray;
use bevy::ecs::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{BindGroup, BindGroupEntry, BindingResource};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::GpuImage;

// INFO: -------------------
//         resources
// -------------------------

/// The layout resource for the global texture map (@group(2)).
#[derive(Resource)]
pub struct TextureArrayBindGroupLayout {
    pub layout: bevy::render::render_resource::BindGroupLayout,
    pub descriptor: bevy::render::render_resource::BindGroupLayoutDescriptor,
}

impl FromWorld for TextureArrayBindGroupLayout {
    #[instrument(skip_all)]
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let descriptor = bevy::render::render_resource::BindGroupLayoutDescriptor {
            label: "Texture Map Bind Group Layout".into(),
            entries: vec![
                // binding 0: texture array
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                    ty: bevy::render::render_resource::BindingType::Texture {
                        sample_type: bevy::render::render_resource::TextureSampleType::Float {
                            filterable: true,
                        },
                        view_dimension:
                            bevy::render::render_resource::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 1: sampler
                bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: bevy::render::render_resource::ShaderStages::FRAGMENT,
                    ty: bevy::render::render_resource::BindingType::Sampler(
                        bevy::render::render_resource::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
        };

        let layout =
            device.create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);

        Self { layout, descriptor }
    }
}

/// A resource owning the GPU data for the texture array for all voxel textures.
#[derive(Resource)]
pub struct TextureArrayUniforms {
    pub bind_group: BindGroup,
}

/// A system that prepares the GPU texture array bind group once the native Bevy Image is extracted.
#[instrument(skip_all)]
pub fn prepare_texture_array_system(
    mut commands: Commands,
    block_texture_array: Option<Res<BlockTextureArray>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    existing: Option<Res<TextureArrayUniforms>>,
    device: Res<RenderDevice>,
    layout: Res<TextureArrayBindGroupLayout>,
) {
    // only run if we have the handle and don't yet have the uniforms
    if let (Some(array_res), None) = (block_texture_array, existing)
        && let Some(gpu_image) = gpu_images.get(array_res.handle.id())
    {
        info!("Initializing GPU texture array bind group from native Bevy Image...");

        // set up the bind group using Bevy's automatically created view and sampler
        let bind_group = device.create_bind_group(
            Some("Texture Map Bind Group"),
            &layout.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gpu_image.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&gpu_image.sampler),
                },
            ],
        );

        commands.insert_resource(TextureArrayUniforms { bind_group });
    }
}
