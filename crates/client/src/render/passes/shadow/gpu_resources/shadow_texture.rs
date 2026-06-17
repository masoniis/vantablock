use crate::prelude::*;
use bevy::{
    ecs::prelude::*,
    render::{
        render_resource::{
            AddressMode, CompareFunction, Extent3d, FilterMode, Sampler, SamplerDescriptor,
            Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            TextureView,
        },
        renderer::RenderDevice,
    },
};

pub const SHADOW_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const SHADOW_MAP_RESOLUTION: u32 = 2048;

/// A resource to hold the shadow depth texture and its view
#[derive(Resource)]
pub struct ShadowDepthTextureResource {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl FromWorld for ShadowDepthTextureResource {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        debug!(
            target : "wgpu_setup",
            "Creating shadow depth texture resource with fixed size {}x{}",
            SHADOW_MAP_RESOLUTION,
            SHADOW_MAP_RESOLUTION
        );

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Shadow Depth Texture"),
            size: Extent3d {
                width: SHADOW_MAP_RESOLUTION,
                height: SHADOW_MAP_RESOLUTION,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: SHADOW_DEPTH_FORMAT,
            // texture binding for sampling in shaders (shadow map)
            // render attachment for rendering to it (shadow pass)
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[SHADOW_DEPTH_FORMAT],
        });

        let view = texture.create_view(&Default::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Shadow Map Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}
