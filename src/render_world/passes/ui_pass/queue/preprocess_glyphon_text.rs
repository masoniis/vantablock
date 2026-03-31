use super::super::gpu_resources::{
    GlyphonAtlasResource, GlyphonCacheResource, GlyphonFontSystemResource, GlyphonRendererResource,
    GlyphonViewportResource,
};
use crate::prelude::*;
use crate::render_world::{
    graphics_context::resources::{RenderDevice, RenderQueue},
    passes::ui_pass::{
        extract::UiElementKind,
        prepare::UiChanges,
        queue::{batch_ui_elements::UiRenderBatch, IsGlyphonDirty, PreparedUiBatches},
    },
};
use bevy::ecs::prelude::*;
use glyphon::{Buffer, Metrics, TextArea, TextBounds};

/// A conditional system that marks Glyphon as dirty if relevant UI changes occurred.
#[instrument(skip_all)]
pub fn mark_glyphon_dirty_system(
    ui_changes: Res<UiChanges>,
    mut is_glyphon_dirty: ResMut<IsGlyphonDirty>,
) {
    if ui_changes.text_content_change_occured || ui_changes.structural_change_occured {
        is_glyphon_dirty.0 = true;
    }
}

/// Preprocesses all UI text for rendering by shaping it and preparing it with the Glyphon renderer.
///
/// This is a CPU-intensive system that should be run before the main render graph execution. It populates the
/// internal buffers of the GlyphonRenderer, which are then used by the UiPassNode to issue the final draw commands.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn preprocess_glyphon_text_system(
    // Input
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    ui_queue: Res<PreparedUiBatches>,

    // Output (update glyphon resources)
    mut font_system: ResMut<GlyphonFontSystemResource>,
    mut cache: ResMut<GlyphonCacheResource>,
    mut atlas: ResMut<GlyphonAtlasResource>,
    viewport: ResMut<GlyphonViewportResource>,
    mut renderer: ResMut<GlyphonRendererResource>,

    mut is_glyphon_dirty: ResMut<IsGlyphonDirty>, // sets flag to false
) {
    debug!(
        target : "ui_efficiency",
        "Preprocessing Glyphon text (this should only happen when UI text changes)..."
    );

    // iterate over text batches only
    for batch in ui_queue.batches.iter() {
        if let UiRenderBatch::Text(text_batch) = batch {
            // create buffers that live for the scope of this function
            let buffers: Vec<Buffer> = text_batch
                .texts
                .iter()
                .map(|text_kind| {
                    if let UiElementKind::Text {
                        content,
                        bounds,
                        font_size,
                        align,
                        ..
                    } = text_kind
                    {
                        let mut buffer =
                            Buffer::new(&mut font_system, Metrics::new(*font_size, *font_size));

                        let attrs = glyphon::Attrs::new().family(glyphon::Family::Name("Miracode"));
                        buffer.set_rich_text(
                            &mut font_system,
                            [(content.as_str(), attrs.clone())],
                            &attrs,
                            glyphon::Shaping::Advanced,
                            Some(*align),
                        );

                        buffer.set_size(&mut font_system, Some(bounds.x), Some(bounds.y));
                        buffer
                    } else {
                        unreachable!(); // shouldn't happen if the batch is correctly identified as Text
                    }
                })
                .collect();

            // create TextAreas that borrow from the local buffers
            let text_areas: Vec<TextArea> = buffers
                .iter()
                .zip(text_batch.texts.iter())
                .map(|(buffer, text_kind)| {
                    if let UiElementKind::Text {
                        position,
                        bounds,
                        color,
                        ..
                    } = text_kind
                    {
                        let text_color = glyphon::Color::rgba(
                            (color[0] * 255.0) as u8,
                            (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8,
                            (color[3] * 255.0) as u8,
                        );

                        TextArea {
                            buffer,
                            left: position.x,
                            top: position.y,
                            scale: 1.0,
                            bounds: TextBounds {
                                left: position.x as i32,
                                top: position.y as i32,
                                right: (position.x + bounds.x) as i32,
                                bottom: (position.y + bounds.y) as i32,
                            },
                            default_color: text_color,
                            custom_glyphs: &[],
                        }
                    } else {
                        unreachable!();
                    }
                })
                .collect();

            renderer
                .prepare(
                    &device,
                    &queue,
                    &mut font_system,
                    &mut atlas,
                    &viewport,
                    text_areas,
                    &mut cache,
                )
                .unwrap();
        }
    }

    is_glyphon_dirty.0 = false;
}
