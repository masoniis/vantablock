use crate::{
    lifecycle::state::ClientLifecycleState,
    network::connection::NetworkErrorEvent,
    ui::{
        systems::spawn_menu_camera_system,
        theme::{Face, backdrop, caption, palette, panel, text},
        widgets::{ButtonTone, button},
    },
};
use bevy::{
    input_focus::{AutoFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::Activate,
};

#[derive(Component, Default, Clone)]
pub struct DisconnectedUiRoot;

pub struct DisconnectedUiPlugin;

impl Plugin for DisconnectedUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_disconnected_ui).add_systems(
            OnEnter(ClientLifecycleState::Error),
            spawn_menu_camera_system,
        );
    }
}

fn spawn_disconnected_ui(trigger: On<NetworkErrorEvent>, mut commands: Commands) {
    let reason = trigger.event().reason.clone();
    info!("Spawning Disconnected UI (Reason: {})...", reason);
    commands
        .spawn_scene(disconnected(&reason))
        .insert(DespawnOnExit(ClientLifecycleState::Error));
}

fn disconnected(reason: &str) -> impl Scene + use<> {
    let reason = reason.to_owned();
    bsn! {
        DisconnectedUiRoot
        @backdrop()
        TabGroup
        Children [
            @panel()
            Node { width: px(460) }
            Children [
                @text("Disconnected", Face::Strong, 22.0, palette::DANGER)
                --
                @caption("Reason")
                --
                @text(&reason, Face::Mono, 14.0, palette::TEXT)
                Node { margin: UiRect::bottom(px(8)) }
                --
                @button("Back to menu", ButtonTone::Normal)
                AutoFocus
                on(|_: On<Activate>, mut next: ResMut<NextState<ClientLifecycleState>>| {
                    next.set(ClientLifecycleState::MainMenu);
                })
            ]
        ]
    }
}
