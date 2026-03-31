use crate::{
    prelude::*,
    simulation_world::{
        input::resources::WindowSizeResource,
        user_interface::{
            components::{self as simulation, CalculatedLayout, UiRoot, UiText},
            layout::IsLayoutDirty,
            screens::spawn_root::UiRootNodeResource,
            text::setup_font::FontSystemResource,
        },
    },
};
use bevy::ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use std::collections::HashMap;
use taffy::{self, TaffyTree};

// INFO: -------------------
//         Resources
// -------------------------

/// The Taffy tree that represents the UI layout.
///
/// It is to be instantiated as a NonSend resource because
/// Taffy is not Send/Sync, unfortunately.
#[derive(Deref, DerefMut, Debug)]
pub struct UiLayoutTree(pub TaffyTree<Entity>);

impl Default for UiLayoutTree {
    fn default() -> Self {
        Self(TaffyTree::new())
    }
}

/// A map from our ECS entities to Taffy node IDs.
#[derive(Resource, Default, Deref, DerefMut, Debug)]
pub struct EntityToNodeMap(pub HashMap<Entity, taffy::NodeId>);

// INFO: -----------------------------------
//         Actual layout computation
// -----------------------------------------

/// A system that computes the layout using Taffy and applies the results back to the ECS entities.
///
/// This is an expensive system that should only be run when the layout is "dirty". It also is an
/// exclusive system because the taffy tree is a NonSend resource.
#[instrument(skip_all)]
pub fn compute_and_apply_layout_system(world: &mut World) {
    debug!(target: "ui_efficiency", "Recomputing the UI layout because it is dirty...");

    // Get the viewport size and root node
    let (root_entity, root_node, viewport_size) = {
        let root_entity = world.get_resource::<UiRootNodeResource>().unwrap().0;
        let entity_to_node = world.resource::<EntityToNodeMap>();
        let window_size = world.resource::<WindowSizeResource>();

        let root_node = entity_to_node[&root_entity];
        let viewport_size = taffy::Size {
            width: taffy::AvailableSpace::Definite(window_size.width as f32),
            height: taffy::AvailableSpace::Definite(window_size.height as f32),
        };

        (root_entity, root_node, viewport_size)
    };

    // Compute the layout (borrowing taffy tree mutably)
    let mut ui_tree = world.remove_non_send_resource::<UiLayoutTree>().unwrap();
    ui_tree
        .compute_layout_with_measure(
            root_node,
            viewport_size,
            |known_dimensions, available_space, _node_id, node_context, _style| {
                if let Some(entity) = node_context {
                    let text = match world.get::<simulation::UiText>(*entity) {
                        Some(text) => text.clone(),
                        None => return taffy::Size::ZERO,
                    };

                    return world.get_resource_mut::<FontSystemResource>().map_or(
                        taffy::Size::ZERO,
                        |mut font_system_res| {
                            measure_text_node(
                                known_dimensions,
                                available_space,
                                &text,
                                &mut font_system_res.font_system,
                            )
                        },
                    );
                }
                taffy::Size::ZERO
            },
        )
        .unwrap();

    // Get all layouts computed with absolute position
    let layouts_to_apply: Vec<(Entity, CalculatedLayout)> = {
        let entity_to_node = world.resource::<EntityToNodeMap>();
        let mut results = Vec::new();

        collect_layouts_recursively(
            &ui_tree,
            entity_to_node,
            root_entity,
            Vec2::ZERO,
            &mut results,
        );
        results
    };

    // And apply those to the world
    for (entity, calculated_layout) in layouts_to_apply {
        let absolute_pos = calculated_layout.position;
        let size = calculated_layout.size;

        if world.get::<UiText>(entity).is_some() {
            debug!(
                target: "ui_layout",
                "[Layout] Text Entity {:?}: abs_pos=({},{}), size=({},{})",
                entity, absolute_pos.x, absolute_pos.y, size.x, size.y
            );
        } else if world.get::<UiRoot>(entity).is_some() {
            debug!(
                target: "ui_layout",
                "[Layout] Root Entity {:?}: abs_pos=({},{}), size=({},{})",
                entity, absolute_pos.x, absolute_pos.y, size.x, size.y
            );
        } else {
            debug!(
                target: "ui_layout",
                "[Layout] UI Entity {:?}: abs_pos=({},{}), size=({},{})",
                entity, absolute_pos.x, absolute_pos.y, size.x, size.y
            );
        }

        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            // update or insert the CalculatedLayout component
            if let Some(mut existing_layout) = entity_mut.get_mut::<CalculatedLayout>() {
                if *existing_layout != calculated_layout {
                    *existing_layout = calculated_layout;
                }
            } else {
                entity_mut.insert(calculated_layout);
            }
        }
    }

    // Put the NonSend resource back and reset dirty state
    world.insert_non_send_resource(ui_tree);
    let mut is_dirty = world.resource_mut::<IsLayoutDirty>();
    is_dirty.0 = false;
}

/// Reads layout data and collects it recursively.
///
/// Does so without modifying the world as not to conflict with other borrows.
fn collect_layouts_recursively(
    ui_tree: &taffy::TaffyTree<Entity>,
    entity_to_node_map: &EntityToNodeMap,
    entity: Entity,
    parent_absolute_pos: Vec2,
    results: &mut Vec<(Entity, CalculatedLayout)>,
) {
    let Some(&node_id) = entity_to_node_map.get(&entity) else {
        return;
    };
    let Ok(layout) = ui_tree.layout(node_id) else {
        return;
    };

    let relative_pos = Vec2::new(layout.location.x, layout.location.y);
    let absolute_pos = parent_absolute_pos + relative_pos;

    let calculated_layout = CalculatedLayout {
        position: absolute_pos,
        size: Vec2::new(layout.size.width, layout.size.height),
    };

    // Add the result to our list instead of inserting into the world.
    results.push((entity, calculated_layout));

    // Recurse for children, getting them from Taffy which is the source of truth for layout.
    if let Ok(children) = ui_tree.children(node_id) {
        // We need to find the entity associated with each child NodeId
        for child_node_id in children {
            if let Some((child_entity, _)) =
                entity_to_node_map.iter().find(|(_, &n)| n == child_node_id)
            {
                collect_layouts_recursively(
                    ui_tree,
                    entity_to_node_map,
                    *child_entity,
                    absolute_pos,
                    results,
                );
            }
        }
    }
}

// INFO: --------------------------
//         conversion utils
// --------------------------------

/// Conversion from the simulationworld component Style to Taffy's Style
impl From<&simulation::Style> for taffy::Style {
    fn from(value: &simulation::Style) -> Self {
        let to_dim = |size: simulation::Size| -> taffy::Dimension {
            match size {
                simulation::Size::Px(px) => taffy::Dimension::length(px),
                simulation::Size::Percent(percent) => taffy::Dimension::percent(percent / 100.0),
                simulation::Size::Auto => taffy::Dimension::auto(),
            }
        };

        taffy::style::Style {
            size: taffy::Size {
                width: to_dim(value.width),
                height: to_dim(value.height),
            },
            padding: taffy::geometry::Rect {
                left: taffy::LengthPercentage::length(value.padding),
                right: taffy::LengthPercentage::length(value.padding),
                top: taffy::LengthPercentage::length(value.padding),
                bottom: taffy::LengthPercentage::length(value.padding),
            },
            position: value.position,
            justify_content: value.justify_content,
            align_items: value.align_items,
            flex_direction: value.flex_direction,
            ..Default::default()
        }
    }
}

/// Measures the size of a text node using the provided FontSystem.
///
/// Note: Text alignment doesn't show up here because alignment does
/// not impact the size of measaurement.
fn measure_text_node(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<taffy::AvailableSpace>,
    text: &simulation::UiText,
    font_system: &mut glyphon::FontSystem,
) -> taffy::Size<f32> {
    // constraints
    let width_constraint = known_dimensions.width.or(match available_space.width {
        taffy::AvailableSpace::MinContent => Some(0.0),
        taffy::AvailableSpace::MaxContent => None,
        taffy::AvailableSpace::Definite(width) => Some(width),
    });

    let height_constraint = known_dimensions.height.or(match available_space.height {
        taffy::AvailableSpace::Definite(height) => Some(height),
        _ => None,
    });

    // shape the text
    let mut buffer = glyphon::Buffer::new(
        font_system,
        glyphon::Metrics::new(text.font_size, text.font_size),
    );
    buffer.set_size(font_system, width_constraint, height_constraint);
    buffer.set_text(
        font_system,
        &text.content,
        &glyphon::Attrs::new().family(glyphon::Family::Name("Miracode")),
        glyphon::Shaping::Advanced,
    );
    buffer.shape_until_scroll(font_system, false);

    // calculate metrics (width/height)
    let measured_width = buffer
        .layout_runs()
        .fold(0.0f32, |max_width, run| max_width.max(run.line_w));

    let line_height = buffer.metrics().line_height;
    let measured_height = buffer.lines.len() as f32 * line_height;

    let final_height = if text.content.is_empty() {
        0.0
    } else {
        measured_height
    };

    debug!(
        target: "ui_layout",
        "[Measure] Text '{}': final_size=({}, {}), available_space=({:?},{:?})",
        text.content,
        measured_width,
        final_height,
        available_space.width,
        available_space.height
    );

    taffy::Size {
        width: measured_width.ceil(),
        height: final_height.ceil(),
    }
}
