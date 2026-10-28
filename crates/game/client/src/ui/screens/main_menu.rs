use crate::{
    lifecycle::state::ClientLifecycleState,
    network::connection::{ConnectType, InitiateConnection},
    ui::{
        systems::spawn_menu_camera_system,
        theme::{Face, backdrop, caption, palette, text},
        widgets::{ButtonTone, button, text_field},
    },
};
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    input_focus::{FocusedInput, tab_navigation::TabGroup},
    picking::Pickable,
    prelude::*,
    text::{EditableText, LetterSpacing},
    ui_widgets::Activate,
};
use common::{
    network::DEFAULT_LOCAL_SERVER_ADDR,
    world::terrain::{TerrainShaperKind, WorldGenSettings},
};

#[derive(Component, Default, Clone)]
pub struct MainMenuUiRoot;

/// The server address field, read when connecting.
#[derive(Component, Default, Clone)]
pub struct ServerAddrInput;

/// The settings the new world page will create a world with.
#[derive(Resource, Default)]
pub struct NewWorldSettings(pub WorldGenSettings);

/// A terrain choice on the new world page.
#[derive(Component, Clone, Default)]
pub struct ShaperOption(pub TerrainShaperKind);

pub struct MainMenuUiPlugin;

impl Plugin for MainMenuUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewWorldSettings>();
        app.add_systems(
            OnEnter(ClientLifecycleState::MainMenu),
            (spawn_menu_camera_system, spawn_main_menu),
        );
        app.add_systems(
            Update,
            mark_selected_shaper.run_if(in_state(ClientLifecycleState::MainMenu)),
        );
    }
}

fn spawn_main_menu(mut commands: Commands) {
    info!("Spawning Main Menu UI...");
    show_page(&mut commands, None, home_page());
}

/// Replaces the open main menu page (if any) with one showing `content` in the menu column.
fn show_page(commands: &mut Commands, open: Option<Entity>, content: Box<dyn Scene>) {
    if let Some(open) = open {
        commands.entity(open).despawn();
    }
    commands
        .spawn_scene(menu_layout(content))
        .insert(DespawnOnExit(ClientLifecycleState::MainMenu));
}

/// The menu frame every page shares: the backdrop, a rail on the left holding the page's
/// column, and the footer.
fn menu_layout(content: Box<dyn Scene>) -> impl Scene {
    bsn! {
        MainMenuUiRoot
        @backdrop()
        Node { align_items: AlignItems::FlexStart, padding: UiRect::left(px(56)) }
        TabGroup
        Children [
            // a rail behind the column, separating it from the rest of the screen
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: px(392),
                height: percent(100),
                border: UiRect::right(px(1)),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.52))
            BorderColor::all(palette::PANEL_BORDER)
            Pickable::IGNORE
            --
            Node { width: px(280), flex_direction: FlexDirection::Column, row_gap: px(10) }
            Children [ @content ]
            --
            @footer()
        ]
    }
}

fn home_page() -> Box<dyn Scene> {
    Box::new(bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(10) }
        Children [
            @title()
            --
            @button("Singleplayer", ButtonTone::Primary)
            Node { margin: UiRect::top(px(28)) }
            on(|_: On<Activate>, mut commands: Commands, page: Single<Entity, With<MainMenuUiRoot>>| {
                show_page(&mut commands, Some(*page), new_world_page());
            })
            --
            @caption("Server address")
            Node { margin: UiRect::top(px(22)) }
            --
            Node { column_gap: px(8), align_items: AlignItems::Center }
            Children [
                @text_field(DEFAULT_LOCAL_SERVER_ADDR, 64)
                ServerAddrInput
                Node { flex_grow: 1.0, min_width: px(0) }
                on(join_on_enter)
                --
                @button("Join", ButtonTone::Normal)
                on(|_: On<Activate>, addr: Single<&EditableText, With<ServerAddrInput>>, mut commands: Commands| {
                    join(&mut commands, &addr);
                })
            ]
        ]
    })
}

/// Settings for a new singleplayer world. Creating it starts the local server with them.
fn new_world_page() -> Box<dyn Scene> {
    let options: Vec<Box<dyn Scene>> = TerrainShaperKind::ALL
        .into_iter()
        .map(|shaper| -> Box<dyn Scene> {
            Box::new(bsn! {
                @button(shaper.display_name(), ButtonTone::Quiet)
                ShaperOption(shaper)
                on(move |_: On<Activate>, mut settings: ResMut<NewWorldSettings>| {
                    settings.0.shaper = shaper;
                })
            })
        })
        .collect();

    Box::new(bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(10) }
        Children [
            @text("New world", Face::Strong, 24.0, palette::TEXT)
            --
            @caption("Terrain")
            Node { margin: UiRect::top(px(14)) }
            --
            Node { flex_direction: FlexDirection::Column, row_gap: px(4) }
            Children [ {options} ]
            --
            @button("Create world", ButtonTone::Primary)
            Node { margin: UiRect::top(px(18)) }
            // singleplayer always runs its own background server
            on(|_: On<Activate>, mut commands: Commands, settings: Res<NewWorldSettings>| {
                commands.trigger(InitiateConnection {
                    connect_type: ConnectType::Singleplayer(settings.0),
                    server_addr: DEFAULT_LOCAL_SERVER_ADDR.to_owned(),
                });
            })
            --
            @button("Back", ButtonTone::Quiet)
            on(|_: On<Activate>, mut commands: Commands, page: Single<Entity, With<MainMenuUiRoot>>| {
                show_page(&mut commands, Some(*page), home_page());
            })
        ]
    })
}

/// Fills the selected terrain option and leaves the others quiet.
fn mark_selected_shaper(
    settings: Res<NewWorldSettings>,
    mut options: Query<(&ShaperOption, &mut ButtonTone)>,
) {
    for (option, mut tone) in &mut options {
        let selected = option.0 == settings.0.shaper;
        tone.set_if_neq(if selected {
            ButtonTone::Normal
        } else {
            ButtonTone::Quiet
        });
    }
}

/// The wordmark: a heavy title, small and left-aligned.
fn title() -> impl Scene {
    bsn! {
        @text("VANTABLOCK", Face::Display, 34.0, palette::TEXT)
        LetterSpacing::Px(4.0)
        TextShadow { offset: Vec2 { x: 0.0, y: 3.0 }, color: {palette::ACCENT_DEEP} }
    }
}

fn footer() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            right: px(16),
            bottom: px(12),
        }
        Children [
            @text(concat!("v", env!("CARGO_PKG_VERSION")), Face::Mono, 12.0, palette::TEXT_FAINT)
        ]
    }
}

/// Pressing Enter in the address field joins that server.
fn join_on_enter(
    key: On<FocusedInput<KeyboardInput>>,
    addr: Single<&EditableText, With<ServerAddrInput>>,
    mut commands: Commands,
) {
    if key.input.key_code == KeyCode::Enter && key.input.state == ButtonState::Pressed {
        join(&mut commands, &addr);
    }
}

fn join(commands: &mut Commands, addr: &EditableText) {
    commands.trigger(InitiateConnection {
        connect_type: ConnectType::Multiplayer,
        server_addr: addr.value().to_string().trim().to_owned(),
    });
}
