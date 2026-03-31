use crate::prelude::*;
use crate::simulation_world::player::CameraComponent;
use crate::simulation_world::terrain::ActiveTerrainGenerator;
use crate::simulation_world::{
    chunk::ChunkCoord,
    time::FrameClock,
    user_interface::{
        components::{Node, Size, Style, TextAlign, UiBackground, UiText},
        screens::spawn_root::UiRootNodeResource,
        screens::MeshCounterResource,
    },
};
use bevy::ecs::{prelude::*, relationship::RelatedSpawnerCommands};

// INFO: -------------------------
//         Marker elements
// -------------------------------

/// An enum representing all possible statistic text markers.
pub enum StatMarker {
    // information
    CameraXYZ(CameraXYZCoordTextMarker),
    CameraChunk(CameraChunkCoordTextMarker),
    Biome(CurrentBiomeTextMarker),
    ActiveGen(ActiveGenTextMarker),
    // performance
    Fps(FpsCounterTextElementMarker),
    Memory(MemoryCounterTextElementMarker),
    MeshCount(MeshCountTextMarker),
    FaceCount(FaceCountTextMarker),
}

/// A marker component for all entities that are part of the diag UI.
#[derive(Component)]
pub struct RootDiagnosticScreenMarker;

/// A marker component for the camera precise coordinate text element.
#[derive(Component)]
pub struct CameraXYZCoordTextMarker;

/// A marker component for the camera XYZ text element.
#[derive(Component)]
pub struct CameraChunkCoordTextMarker;

/// A marker component for the current biome text element.
#[derive(Component)]
pub struct CurrentBiomeTextMarker;

/// A marker component for the active generator text element.
#[derive(Component)]
pub struct ActiveGenTextMarker;

/// A marker component for the FPS counter text element.
#[derive(Component)]
pub struct FpsCounterTextElementMarker;

/// A marker component for the memory counter text element.
#[derive(Component)]
pub struct MemoryCounterTextElementMarker;

/// A marker component for the total mesh count text element.
#[derive(Component)]
pub struct MeshCountTextMarker;

/// A marker component for the total vertex count text element.
#[derive(Component)]
pub struct FaceCountTextMarker;

// INFO: -------------------------------------
//         Toggling and creation logic
// -------------------------------------------

/// A run condition that returns true if the diagnostic UI is currently spawned and visible.
pub fn diagnostic_ui_is_visible(query: Query<(), With<RootDiagnosticScreenMarker>>) -> bool {
    !query.is_empty()
}

/// Toggles the debug diagnostics UI by spawning or despawning it.
#[instrument(skip_all)]
pub fn toggle_debug_diagnostics_system(
    // Input
    root_node: Res<UiRootNodeResource>,
    query: Query<Entity, With<RootDiagnosticScreenMarker>>,

    // Needed for init values on spawn
    mesh_stats: Res<MeshCounterResource>,
    camera_query: Query<(&CameraComponent, &ChunkCoord)>,
    time_stats: Res<FrameClock>,
    active_gen: Res<ActiveTerrainGenerator>,

    // Output (toggling UI)
    mut commands: Commands,
) {
    if let Ok(ui_entity) = query.single() {
        info!("Despawning Diagnostic UI...");
        commands.entity(ui_entity).despawn();
    } else if let Ok((cam, chord)) = camera_query.single() {
        spawn_diagnostic_ui(
            &mut commands,
            &root_node,
            &mesh_stats,
            cam,
            chord,
            &time_stats,
            &active_gen,
        );
    } else {
        error!("Cannot spawn Diagnostic UI: No camera with ChunkChord found!");
    }
}

/// Spawns the entire Diagnostic UI and attaches it to the persistent root node.
fn spawn_diagnostic_ui(
    commands: &mut Commands,
    root_node: &Res<UiRootNodeResource>,

    // init stats
    mesh_stats: &Res<MeshCounterResource>,
    camera: &CameraComponent,
    camera_chord: &ChunkCoord,
    time_stats: &Res<FrameClock>,
    active_gen: &Res<ActiveTerrainGenerator>,
) {
    info!("Spawning Diagnostic UI...");
    let root_entity = root_node.0;

    let diagnostic_ui_container = commands
        .spawn((
            RootDiagnosticScreenMarker,
            Node,
            Style {
                position: taffy::style::Position::Absolute,
                width: Size::Percent(100.0),
                height: Size::Percent(100.0),
                flex_direction: taffy::style::FlexDirection::Row,
                justify_content: Some(taffy::JustifyContent::SpaceBetween),
                align_items: Some(taffy::AlignItems::Start),
                ..Default::default()
            },
        ))
        .with_children(|parent| {
            let font_size = 32.0;
            let align = TextAlign::Center;

            // INFO: wrapper for elements on left side of screen
            parent
                .spawn((
                    Node,
                    Style {
                        width: Size::Percent(100.0),
                        height: Size::Percent(100.0),
                        flex_direction: taffy::style::FlexDirection::Column,
                        justify_content: Some(taffy::JustifyContent::Start),
                        align_items: Some(taffy::AlignItems::Start),
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    // precise coord line
                    let precise_coord_line_elements = vec![StatLineElement {
                        prefix: "XYZ coord: ".to_string(),
                        content: format!(
                            "{:.2}, {:.2}, {:.2}",
                            camera.position.x, camera.position.y, camera.position.z
                        ), // initial value
                        color: [0.8, 0.8, 0.8, 1.0],
                        marker: StatMarker::CameraXYZ(CameraXYZCoordTextMarker),
                    }];
                    spawn_stats_line(parent, precise_coord_line_elements, font_size, align);

                    // chunk chord line
                    let chord_line_elements = vec![StatLineElement {
                        prefix: "Chunk coord: ".to_string(),
                        content: camera_chord.to_string(),
                        color: [0.8, 0.8, 0.2, 1.0],
                        marker: StatMarker::CameraChunk(CameraChunkCoordTextMarker),
                    }];
                    spawn_stats_line(parent, chord_line_elements, font_size, align);

                    // biome line
                    let biome_line_elements = vec![StatLineElement {
                        prefix: "Biome: ".to_string(),
                        content: "Unknown".to_string(), // initial value
                        color: [0.2, 0.8, 0.2, 1.0],
                        marker: StatMarker::Biome(CurrentBiomeTextMarker),
                    }];
                    spawn_stats_line(parent, biome_line_elements, font_size, align);

                    // active generator line
                    let gen_line_elements = vec![StatLineElement {
                        prefix: "Active gen: ".to_string(),
                        content: active_gen.0.name().to_string(),
                        color: [0.2, 0.8, 0.8, 1.0],
                        marker: StatMarker::ActiveGen(ActiveGenTextMarker),
                    }];
                    spawn_stats_line(parent, gen_line_elements, font_size, align);
                });

            // INFO: wrapper for elements on right side of screen
            parent
                .spawn((
                    Node,
                    Style {
                        width: Size::Percent(100.0),
                        height: Size::Percent(100.0),
                        flex_direction: taffy::style::FlexDirection::Column,
                        justify_content: Some(taffy::JustifyContent::Start),
                        align_items: Some(taffy::AlignItems::End),
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    // fps line
                    let fps_line_elements = vec![StatLineElement {
                        prefix: "FPS: ".to_string(),
                        content: format!("{:.2}", time_stats.smoothed_fps),
                        color: [1.0, 1.0, 1.0, 1.0],
                        marker: StatMarker::Fps(FpsCounterTextElementMarker),
                    }];
                    spawn_stats_line(parent, fps_line_elements, font_size, align);

                    let memory_line_elements = vec![StatLineElement {
                        prefix: "MEM: ".to_string(),
                        content: "0 MB".to_string(),
                        color: [1.0, 1.0, 1.0, 1.0],
                        marker: StatMarker::Memory(MemoryCounterTextElementMarker),
                    }];
                    spawn_stats_line(parent, memory_line_elements, font_size, align);

                    // mesh line
                    let mesh_line_elements = vec![
                        StatLineElement {
                            prefix: "Meshes: ".to_string(),
                            content: mesh_stats.total_meshes.to_string(),
                            color: [0.9, 0.6, 0.6, 1.0],
                            marker: StatMarker::MeshCount(MeshCountTextMarker),
                        },
                        StatLineElement {
                            prefix: " Faces: ".to_string(),
                            content: mesh_stats.total_faces.to_string(),
                            color: [0.6, 0.8, 0.6, 1.0],
                            marker: StatMarker::FaceCount(FaceCountTextMarker),
                        },
                    ];
                    spawn_stats_line(parent, mesh_line_elements, font_size, align);
                });
        })
        .id();

    commands
        .entity(root_entity)
        .add_child(diagnostic_ui_container);
}

/// A data struct to define one part of a multi-part stat line.
pub struct StatLineElement {
    /// A label prefix for the dynamic text (e.g., "FPS: ")
    pub prefix: String,
    /// The initial value for the dynamic text (e.g., "0")
    pub content: String,
    /// The color of the dynamic text
    pub color: [f32; 4],
    /// The marker component, wrapped in our enum.
    pub marker: StatMarker,
}

/// A generic helper to spawn a multi-part statistics line from a Vec of elements.
fn spawn_stats_line(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    elements: Vec<StatLineElement>,
    font_size: f32,
    text_align: TextAlign,
) {
    parent
        .spawn((
            Node,
            Style {
                padding: 8.0,
                flex_direction: taffy::style::FlexDirection::Row,
                align_items: Some(taffy::style::AlignItems::Center),
                ..Default::default()
            },
            UiBackground::SolidColor {
                color: [0.0, 0.0, 0.0, 0.33],
            },
        ))
        .with_children(|line| {
            let static_color = [0.7, 0.7, 0.7, 1.0];

            for element in elements {
                // static prefix
                if !element.prefix.is_empty() {
                    line.spawn((
                        Node,
                        Style::default(),
                        UiText {
                            content: element.prefix,
                            font_size,
                            color: static_color,
                            align: text_align,
                        },
                    ));
                }

                // dynamic text with marker
                let mut text_entity = line.spawn((
                    Node,
                    Style::default(),
                    UiText {
                        content: element.content,
                        font_size,
                        color: element.color,
                        align: text_align,
                    },
                ));
                match element.marker {
                    StatMarker::CameraXYZ(marker) => text_entity.insert(marker),
                    StatMarker::CameraChunk(marker) => text_entity.insert(marker),
                    StatMarker::Biome(marker) => text_entity.insert(marker),
                    StatMarker::ActiveGen(marker) => text_entity.insert(marker),
                    StatMarker::Fps(marker) => text_entity.insert(marker),
                    StatMarker::Memory(marker) => text_entity.insert(marker),
                    StatMarker::MeshCount(marker) => text_entity.insert(marker),
                    StatMarker::FaceCount(marker) => text_entity.insert(marker),
                };
            }
        });
}
