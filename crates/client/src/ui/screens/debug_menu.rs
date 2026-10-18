use crate::{input::local_actions::ClientAction, player::LocalPlayer};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use leafwing_input_manager::common_conditions::action_just_pressed;
use shared::{
    player::components::{LogicalPosition, PlayerLook},
    time::WorldClockResource,
};
use std::fmt::Write;

#[derive(Component)]
pub struct DebugMenuRoot;

#[derive(Component)]
pub struct DebugMenuText;

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

/// Spawns or despawns the menu to match [`DebugMenuState`], however it was changed.
fn sync_debug_menu_system(
    debug_state: Res<DebugMenuState>,
    mut commands: Commands,
    root_query: Query<Entity, With<DebugMenuRoot>>,
    asset_server: Res<AssetServer>,
) {
    for entity in root_query.iter() {
        commands.entity(entity).despawn();
    }
    if !debug_state.visible {
        return;
    }

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
}

fn update_debug_menu_system(
    diagnostics: Res<DiagnosticsStore>,
    player_query: Query<(&LogicalPosition, &PlayerLook), With<LocalPlayer>>,
    clock: Res<WorldClockResource>,
    mut text_query: Query<&mut Text, With<DebugMenuText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.0);

    // writing to a String cannot fail
    let mut text = String::new();
    let _ = writeln!(text, "FPS: {fps:.1}");
    if let Ok((pos, look)) = player_query.single() {
        let _ = writeln!(text, "Pos: {:.2}, {:.2}, {:.2}", pos.0.x, pos.0.y, pos.0.z);
        let _ = writeln!(
            text,
            "Facing: Yaw {:.1}, Pitch {:.1}",
            look.yaw.to_degrees(),
            look.pitch.to_degrees()
        );
    }
    let _ = writeln!(
        text,
        "Time of day: {:.0}%{}",
        clock.day_night_cycle_value() * 100.0,
        if clock.day_cycle_paused { " (paused)" } else { "" }
    );

    for mut t in text_query.iter_mut() {
        **t = text.clone();
    }
}
