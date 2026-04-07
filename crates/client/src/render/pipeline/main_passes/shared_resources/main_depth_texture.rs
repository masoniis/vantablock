use bevy::ecs::prelude::*;
use bevy::render::render_resource::{
    Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::view::ExtractedView;

pub const MAIN_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// A resource to hold the depth texture and its view
#[derive(Resource)]
pub struct MainDepthTextureResource {
    pub texture: Texture,
    pub view: TextureView,
}

impl FromWorld for MainDepthTextureResource {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        // Use a default size; the resize system should be called to set the real size
        Self::new(device, 1, 1)
    }
}

impl MainDepthTextureResource {
    pub fn new(device: &RenderDevice, width: u32, height: u32) -> MainDepthTextureResource {
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: Some("Main Depth Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: MAIN_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[MAIN_DEPTH_FORMAT],
        });

        let depth_view = depth_texture.create_view(&Default::default());

        Self {
            texture: depth_texture,
            view: depth_view,
        }
    }
}

/// A system to resize the main depth texture to match the viewport size.
pub fn resize_main_depth_texture_system(
    mut depth_texture: ResMut<MainDepthTextureResource>,
    device: Res<RenderDevice>,
    views: Query<&ExtractedView>,
) {
    // we assume all views share the same depth texture size for now (usually true for a single window)
    if let Some(view) = views.iter().next() {
        let width = view.viewport.z;
        let height = view.viewport.w;

        if depth_texture.texture.width() != width || depth_texture.texture.height() != height {
            *depth_texture = MainDepthTextureResource::new(&device, width, height);
        }
    }
}
