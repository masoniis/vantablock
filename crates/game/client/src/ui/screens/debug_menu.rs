//! The F3 debug overlay: player state on the left, world and chunk state on the right, each line
//! on its own translucent strip with colored labels and values.

use crate::{
    input::local_actions::ClientAction,
    player::LocalPlayer,
    render::chunk::ChunkMesh,
    ui::theme::{Face, font, palette},
    world::chunk::{ClientChunkMap, ClientChunkState},
};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use common::{
    player::components::{LogicalPosition, PlayerLook},
    world::chunk::{RENDER_DISTANCE, components::ChunkCoord},
};
use leafwing_input_manager::common_conditions::action_just_pressed;

const FONT_SIZE: f32 = 14.0;

#[derive(Component, Default, Clone)]
pub struct DebugMenuRoot;

/// A column of overlay lines, and the span count of each line it currently holds.
#[derive(Component, Default, Clone)]
pub struct DebugColumn {
    side: Side,
    shape: Vec<usize>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Side {
    #[default]
    Left,
    Right,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct DebugMenuState {
    pub visible: bool,
}

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.init_resource::<DebugMenuState>()
            .register_type::<DebugMenuState>()
            .add_systems(
                Update,
                (
                    toggle_debug_menu_system
                        .run_if(action_just_pressed(ClientAction::ToggleDebugMenu)),
                    sync_debug_menu_system.run_if(resource_changed::<DebugMenuState>),
                    update_debug_menu_system.run_if(|state: Res<DebugMenuState>| state.visible),
                )
                    .chain(),
            );
    }
}

fn toggle_debug_menu_system(mut debug_state: ResMut<DebugMenuState>) {
    debug_state.visible = !debug_state.visible;
}

/// Spawns or despawns the overlay to match [`DebugMenuState`], however it was changed.
fn sync_debug_menu_system(
    debug_state: Res<DebugMenuState>,
    mut commands: Commands,
    root_query: Query<Entity, With<DebugMenuRoot>>,
) {
    for entity in root_query.iter() {
        commands.entity(entity).despawn();
    }
    if debug_state.visible {
        commands.spawn_scene(debug_overlay());
    }
}

fn debug_overlay() -> impl Scene {
    bsn! {
        DebugMenuRoot
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            padding: UiRect::all(px(6)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
        }
        // behind the menus, so the pause scrim dims it along with the game
        GlobalZIndex(-1)
        Children [
            (
                template_value(DebugColumn { side: Side::Left, shape: Vec::new() })
                Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexStart }
            ),
            (
                template_value(DebugColumn { side: Side::Right, shape: Vec::new() })
                Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd }
            ),
        ]
    }
}

/// One overlay line, built from colored spans.
#[derive(Default)]
struct Line(Vec<(String, Color)>);

impl Line {
    fn span(mut self, text: impl Into<String>, color: Color) -> Self {
        self.0.push((text.into(), color));
        self
    }
    fn label(self, text: &str) -> Self {
        self.span(format!("{text}: "), palette::LABEL)
    }
    fn value(self, text: impl Into<String>) -> Self {
        self.span(text, palette::TEXT)
    }
    fn dim(self, text: impl Into<String>) -> Self {
        self.span(text, palette::TEXT_DIM)
    }
    fn gap(self) -> Self {
        self.span("  ", palette::TEXT)
    }
}

/// Green when comfortably fast, amber when middling, red when slow.
fn grade(value: f64, good: f64, bad: f64) -> Color {
    // `good < bad` grades a cost, `good > bad` a rate
    let (v, g, b) = if good < bad {
        (value, good, bad)
    } else {
        (-value, -good, -bad)
    };
    if v <= g {
        palette::GOOD
    } else if v <= b {
        palette::WARN
    } else {
        palette::BAD
    }
}

fn update_debug_menu_system(
    diagnostics: Res<DiagnosticsStore>,
    player_query: Query<(&LogicalPosition, &ChunkCoord, &PlayerLook), With<LocalPlayer>>,
    chunk_map: Res<ClientChunkMap>,
    chunk_meshes: Query<&ChunkMesh>,
    mut columns: Query<(Entity, &mut DebugColumn)>,
    mut writer: TextUiWriter,
    children: Query<&Children>,
    mut commands: Commands,
) {
    // INFO: player state, left
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);

    let mut left = vec![
        Line::default()
            .span(
                concat!("Vantablock ", env!("CARGO_PKG_VERSION")),
                palette::HEADING,
            )
            .gap()
            .span(format!("{fps:.0}"), grade(fps, 100.0, 50.0))
            .dim(format!(" fps  {frame_ms:.2} ms")),
    ];

    if let Ok((pos, coord, look)) = player_query.single() {
        left.push(
            Line::default()
                .label("Pos")
                .span(format!("{:.2}", pos.0.x), palette::AXIS_X)
                .gap()
                .span(format!("{:.2}", pos.0.y), palette::AXIS_Y)
                .gap()
                .span(format!("{:.2}", pos.0.z), palette::AXIS_Z),
        );
        left.push(
            Line::default()
                .label("Chunk")
                .span(coord.pos.x.to_string(), palette::AXIS_X)
                .gap()
                .span(coord.pos.y.to_string(), palette::AXIS_Y)
                .gap()
                .span(coord.pos.z.to_string(), palette::AXIS_Z),
        );
        left.push(
            Line::default()
                .label("Facing")
                .dim("yaw ")
                .value(format!("{:.1}", look.yaw.to_degrees()))
                .dim("  pitch ")
                .value(format!("{:.1}", look.pitch.to_degrees())),
        );
    }

    // INFO: world and chunk state, right
    let awaiting = chunk_map
        .chunk_states
        .values()
        .filter(|state| **state == ClientChunkState::AwaitingData)
        .count();
    let (mut meshing, mut needs_meshing) = (0, 0);
    for mesh in &chunk_meshes {
        match mesh {
            ChunkMesh::Meshing => meshing += 1,
            ChunkMesh::Queued => needs_meshing += 1,
            ChunkMesh::Meshed => {}
        }
    }

    let right = vec![
        Line::default()
            .label("Chunks loaded")
            .value(chunk_map.chunk_states.len().to_string()),
        Line::default()
            .label("Render distance")
            .value(RENDER_DISTANCE.to_string()),
        Line::default()
            .label("Awaiting data")
            .span(awaiting.to_string(), grade(awaiting as f64, 0.0, 32.0)),
        Line::default()
            .label("Meshing")
            .value(meshing.to_string())
            .dim(format!("  queue {needs_meshing}")),
    ];

    for (entity, mut column) in &mut columns {
        let lines = match column.side {
            Side::Left => &left,
            Side::Right => &right,
        };
        write_lines(
            entity,
            &mut column,
            lines,
            &mut writer,
            &children,
            &mut commands,
        );
    }
}

/// Writes `lines` into a column, updating the existing spans in place and only respawning the
/// line entities when the column's shape (line and span counts) changed.
fn write_lines(
    entity: Entity,
    column: &mut DebugColumn,
    lines: &[Line],
    writer: &mut TextUiWriter,
    children: &Query<&Children>,
    commands: &mut Commands,
) {
    let shape: Vec<usize> = lines.iter().map(|line| line.0.len()).collect();
    if shape != column.shape {
        column.shape = shape;
        let scenes: Vec<_> = lines.iter().map(line_scene).collect();
        commands
            .entity(entity)
            .despawn_children()
            .queue_spawn_related_scenes::<Children>(scenes);
        return;
    }
    let Ok(rows) = children.get(entity) else {
        return;
    };
    for (row, line) in rows.iter().zip(lines) {
        // the line's root text is empty, its spans start at index 1
        for (i, (text, color)) in line.0.iter().enumerate() {
            if let Some(mut current) = writer.get_text(row, i + 1)
                && *current != *text
            {
                current.clone_from(text);
            }
            if let Some(mut current) = writer.get_color(row, i + 1) {
                current.set_if_neq(TextColor(*color));
            }
        }
    }
}

fn line_scene(line: &Line) -> impl Scene + use<> {
    let spans: Vec<_> = line
        .0
        .iter()
        .map(|(text, color)| {
            let (text, color) = (text.clone(), *color);
            bsn! { TextSpan(text) font(Face::Mono, FONT_SIZE) TextColor(color) }
        })
        .collect();
    bsn! {
        Text("")
        font(Face::Mono, FONT_SIZE)
        Node { padding: UiRect::axes(px(4), px(1)) }
        BackgroundColor(palette::OVERLAY_LINE)
        Children [ {spans} ]
    }
}
