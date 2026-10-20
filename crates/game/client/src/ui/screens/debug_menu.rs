//! The F3 debug overlay: game state on the left, renderer state on the right, each line on its
//! own translucent strip with colored labels and values.

use crate::{
    input::local_actions::ClientAction,
    player::LocalPlayer,
    render::{
        RenderSettings, device::GpuCapabilities, geometry::GeometryLibraryHandle,
        scene::ClientScene,
    },
    ui::theme::{Face, font, palette},
};
use bevy::{
    diagnostic::{DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use common::{
    player::components::{LogicalPosition, PlayerLook},
    time::WorldClockResource,
};
use leafwing_input_manager::common_conditions::action_just_pressed;

/// Render passes whose timings are shown, by diagnostic span name.
const PASS_SPANS: [&str; 3] = [
    "voxel/scene_tlas",
    "voxel/primary_visibility",
    "voxel/direct_lighting",
];

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
    fn mode(self, text: impl Into<String>) -> Self {
        self.span(text, palette::ENUM)
    }
    fn gap(self) -> Self {
        self.span("  ", palette::TEXT)
    }
}

fn heading(text: &str) -> Line {
    Line::default().span(text, palette::HEADING)
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

#[allow(clippy::too_many_arguments)]
fn update_debug_menu_system(
    diagnostics: Res<DiagnosticsStore>,
    player_query: Query<(&LogicalPosition, &PlayerLook), With<LocalPlayer>>,
    clock: Res<WorldClockResource>,
    settings: Res<RenderSettings>,
    scene: Res<ClientScene>,
    gpu: Option<Res<GpuCapabilities>>,
    geometry: Option<Res<GeometryLibraryHandle>>,
    mut columns: Query<(Entity, &mut DebugColumn)>,
    mut writer: TextUiWriter,
    children: Query<&Children>,
    mut commands: Commands,
) {
    // INFO: game state, left
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
    if let Ok((pos, look)) = player_query.single() {
        left.push(
            Line::default()
                .label("XYZ")
                .span(format!("{:.2}", pos.0.x), palette::AXIS_X)
                .dim(" / ")
                .span(format!("{:.2}", pos.0.y), palette::AXIS_Y)
                .dim(" / ")
                .span(format!("{:.2}", pos.0.z), palette::AXIS_Z),
        );
        left.push(
            Line::default()
                .label("Facing")
                .dim("yaw ")
                .value(format!("{:.1}°", look.yaw.to_degrees()))
                .dim("  pitch ")
                .value(format!("{:.1}°", look.pitch.to_degrees())),
        );
    }
    let mut time = Line::default()
        .label("Time of day")
        .value(format!("{:.0}%", clock.day_night_cycle_value() * 100.0));
    if clock.day_cycle_paused {
        time = time.span(" paused", palette::WARN);
    }
    left.push(time);

    // INFO: renderer state, right
    let s = scene.stats;
    let mut right = Vec::new();
    if let Some(gpu) = &gpu {
        right.push(
            Line::default()
                .label("GPU")
                .value(gpu.adapter_name.clone())
                .dim(format!(" ({})", gpu.backend)),
        );
        right.push(Line::default().label("Max binding").value(format!(
            "{} MiB",
            gpu.residency.max_storage_buffer_binding_size >> 20
        )));
    }
    right.push(heading("Renderer"));
    right.push(
        Line::default()
            .label("View")
            .mode(format!("{:?}", settings.debug_view))
            .gap()
            .label("Scale")
            .mode(format!("{}", settings.render_scale)),
    );
    right.push(
        Line::default()
            .label("Acceleration")
            .mode(format!("{:?}", settings.acceleration))
            .gap()
            .label("Hierarchy")
            .mode(format!("{:?}", settings.hierarchy)),
    );
    right.push(
        Line::default()
            .label("Broad phase")
            .mode(if s.hardware_broad_phase {
                "hardware"
            } else {
                "software"
            })
            .gap()
            .label("Footprint LOD")
            .mode(if settings.footprint_lod {
                if settings.footprint_stochastic {
                    format!("on ({} px, stochastic)", settings.footprint_scale)
                } else {
                    format!(
                        "on ({} px, {})",
                        settings.footprint_scale, settings.footprint_threshold
                    )
                }
            } else {
                "off".to_string()
            })
            .gap()
            .label("TAA")
            .mode(if settings.temporal_aa { "on" } else { "off" }),
    );
    right.push(
        Line::default()
            .label("Objects")
            .value(s.objects.to_string())
            .gap()
            .label("Frames")
            .value(s.frames.to_string())
            .gap()
            .label("BVH")
            .value(s.bvh_nodes.to_string()),
    );
    if s.out_of_envelope > 0 || s.overflowed_frames > 0 {
        right.push(
            Line::default()
                .label("Out of envelope")
                .span(s.out_of_envelope.to_string(), palette::BAD)
                .gap()
                .label("Dropped frames")
                .span(s.overflowed_frames.to_string(), palette::BAD),
        );
    }
    right.push(
        Line::default()
            .label("Scene build")
            .span(
                format!("{:.2}", s.build_millis),
                grade(s.build_millis as f64, 1.0, 4.0),
            )
            .dim(" ms"),
    );

    let mut passes = Vec::new();
    for span in PASS_SPANS {
        let read = |field: &str| {
            diagnostics
                .get(&DiagnosticPath::new(format!("render/{span}/{field}")))
                .and_then(|d| d.smoothed())
        };
        let (millis, unit) = match (read("elapsed_gpu"), read("elapsed_cpu")) {
            (Some(gpu), _) => (gpu, " ms gpu"),
            (None, Some(cpu)) => (cpu, " ms cpu (no gpu timestamps)"),
            _ => continue,
        };
        passes.push(
            Line::default()
                .label(span.trim_start_matches("voxel/"))
                .span(format!("{millis:.2}"), grade(millis, 2.0, 8.0))
                .dim(unit),
        );
    }
    if !passes.is_empty() {
        right.push(heading("Passes"));
        right.extend(passes);
    }

    if let Some(geometry) = &geometry {
        let g = geometry.0.stats;
        right.push(heading("Geometry"));
        right.push(Line::default().label("Voxels").value(g.voxels.to_string()));
        right.push(
            Line::default()
                .label("Nodes")
                .value(g.nodes[0].to_string())
                .dim(" 64-tree / ")
                .value(g.nodes[1].to_string())
                .dim(" octree"),
        );
        right.push(
            Line::default()
                .label("Memory")
                .value(format!(
                    "{:.1} MiB",
                    (g.topology_bytes + g.attribute_bytes) as f64 / (1 << 20) as f64
                ))
                .dim(format!(
                    "  {} pages, {} shared blocks",
                    g.topology_pages, g.shared_blocks
                )),
        );
    }

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
