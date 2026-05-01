use crate::{
    input::local_actions::LocalClientAction,
    render::chunk::manager::{ClientChunkManager, ClientChunkState},
};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use leafwing_input_manager::prelude::ActionState;
use shared::{
    player::components::{LogicalPosition, PlayerLook},
    world::chunk::components::ChunkCoord,
};

#[derive(Component)]
pub struct DebugMenuRoot;

#[derive(Component)]
pub struct DebugMenuText;

#[derive(Resource, Default)]
pub struct DebugMenuState {
    pub visible: bool,
}

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.init_resource::<DebugMenuState>().add_systems(
            Update,
            (
                toggle_debug_menu_system,
                update_debug_menu_system.run_if(|state: Res<DebugMenuState>| state.visible),
            ),
        );
    }
}

fn toggle_debug_menu_system(
    mut debug_state: ResMut<DebugMenuState>,
    action_query: Query<&ActionState<LocalClientAction>>,
    mut commands: Commands,
    root_query: Query<Entity, With<DebugMenuRoot>>,
    asset_server: Res<AssetServer>,
) {
    let mut just_pressed = false;
    for action_state in action_query.iter() {
        if action_state.just_pressed(&LocalClientAction::ToggleDebugMenu) {
            just_pressed = true;
            break;
        }
    }

    if just_pressed {
        debug_state.visible = !debug_state.visible;

        if debug_state.visible {
            // Spawn UI
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(10.0),
                        left: Val::Px(10.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                    DebugMenuRoot,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font: asset_server.load("client/font/Recursive_variable.ttf"),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        DebugMenuText,
                    ));
                });
        } else {
            // Despawn UI
            for entity in root_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn update_debug_menu_system(
    diagnostics: Res<DiagnosticsStore>,
    player_query: Query<(&LogicalPosition, &ChunkCoord, &PlayerLook)>,
    chunk_manager: Res<ClientChunkManager>,
    mut text_query: Query<&mut Text, With<DebugMenuText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);

    let (pos, coord, look) = if let Ok((pos, coord, look)) = player_query.single() {
        (pos.0, coord.pos, Some(look))
    } else {
        (Vec3::ZERO, IVec3::ZERO, None)
    };

    let mut text_content = format!(
        "FPS: {:.1}\n\
        Pos: {:.2}, {:.2}, {:.2}\n\
        Chunk: {} {} {}\n",
        fps, pos.x, pos.y, pos.z, coord.x, coord.y, coord.z,
    );

    if let Some(look) = look {
        text_content.push_str(&format!(
            "Facing: Yaw {:.1}, Pitch {:.1}\n",
            look.yaw.to_degrees(),
            look.pitch.to_degrees()
        ));
    }

    text_content.push_str(&format!(
        "Chunks Loaded: {}\n\
        Render Distance: {}\n",
        chunk_manager.chunk_states.len(),
        shared::world::chunk::RENDER_DISTANCE,
    ));

    // count meshing chunks
    let meshing_count = chunk_manager
        .chunk_states
        .values()
        .filter(|s| matches!(s, ClientChunkState::Meshing { .. }))
        .count();
    let needs_meshing_count = chunk_manager
        .chunk_states
        .values()
        .filter(|s| matches!(s, ClientChunkState::NeedsMeshing { .. }))
        .count();

    text_content.push_str(&format!(
        "Meshing: {} (Queue: {})\n",
        meshing_count, needs_meshing_count
    ));

    for mut text in text_query.iter_mut() {
        **text = text_content.clone();
    }
}
