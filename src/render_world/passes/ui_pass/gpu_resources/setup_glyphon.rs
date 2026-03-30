use crate::prelude::*;
use crate::render_world::graphics_context::resources::{
    RenderDevice, RenderQueue, RenderSurfaceConfig,
};
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use glyphon::{fontdb::Source, Cache, FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};
use std::sync::Arc;

#[derive(Resource, Deref, DerefMut)]
pub struct GlyphonFontSystemResource(pub FontSystem);

#[derive(Resource, Deref, DerefMut)]
pub struct GlyphonCacheResource(pub SwashCache);

#[derive(Resource, Deref, DerefMut)]
pub struct GlyphonAtlasResource(pub TextAtlas);

#[derive(Resource, Deref, DerefMut)]
pub struct GlyphonViewportResource(pub Viewport);

#[derive(Resource, Deref, DerefMut)]
pub struct GlyphonRendererResource(pub TextRenderer);

#[instrument(skip_all)]
pub fn setup_glyphon_resources(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    config: Res<RenderSurfaceConfig>,
) {
    let font_bytes = include_bytes!("../../../../../assets/fonts/Miracode.ttf");
    let source = Source::Binary(Arc::new(font_bytes));
    let font_system = FontSystem::new_with_fonts(vec![source]);

    let cache = SwashCache::new();
    let viewport_cache = Cache::new(&device.0);
    let mut viewport = Viewport::new(&device.0, &viewport_cache);
    viewport.update(
        &queue.0,
        glyphon::Resolution {
            width: config.0.width,
            height: config.0.height,
        },
    );
    let mut atlas = TextAtlas::new(&device.0, &queue.0, &viewport_cache, config.0.format);
    let renderer = TextRenderer::new(
        &mut atlas,
        &device.0,
        wgpu::MultisampleState::default(),
        None,
    );

    commands.insert_resource(GlyphonFontSystemResource(font_system));
    commands.insert_resource(GlyphonCacheResource(cache));
    commands.insert_resource(GlyphonAtlasResource(atlas));
    commands.insert_resource(GlyphonViewportResource(viewport));
    commands.insert_resource(GlyphonRendererResource(renderer));
}
