//! Keeps the primary window hidden until its first frame is one we drew.
//!
//! The window starts invisible (see `DefaultClientPlugins`). Showing it as soon as the app ran
//! presented the platform's default background for a few frames, while the UI font loaded and
//! the UI pipelines compiled. It is shown once the render world has drawn a few frames with no
//! pipeline left compiling.

use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSystems, render_resource::PipelineCache},
    window::PrimaryWindow,
};
use common::lifecycle::state::AppState;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

/// Consecutive rendered frames with every pipeline compiled before the window is shown.
const SETTLED_FRAMES: u32 = 3;
/// Shows the window regardless after this long, so a pipeline that never compiles can't hide it.
const REVEAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Consecutive rendered frames in which no pipeline was waiting to compile, counted by the render
/// world and read by the main world.
#[derive(Resource, Clone, Default)]
struct SettledFrames(Arc<AtomicU32>);

pub struct WindowRevealPlugin;

impl Plugin for WindowRevealPlugin {
    fn build(&self, app: &mut App) {
        let settled = SettledFrames::default();
        app.insert_resource(settled.clone()).add_systems(
            Update,
            reveal_window_system.run_if(in_state(AppState::Running)),
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(settled)
                .add_systems(Render, count_settled_frames.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Runs after the pipeline cache has processed this frame's queue.
fn count_settled_frames(cache: Res<PipelineCache>, settled: Res<SettledFrames>) {
    if cache.waiting_pipelines().next().is_none() {
        settled.0.fetch_add(1, Ordering::Relaxed);
    } else {
        settled.0.store(0, Ordering::Relaxed);
    }
}

fn reveal_window_system(
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    settled: Res<SettledFrames>,
    time: Res<Time<Real>>,
    mut counting: Local<bool>,
) {
    let Ok(mut window) = window.single_mut() else {
        return;
    };
    if window.visible {
        return;
    }

    let timed_out = time.elapsed() > REVEAL_TIMEOUT;
    if !*counting {
        // count afresh from the first frame of the running app, so frames drawn during startup
        // (before the UI existed) don't count
        settled.0.store(0, Ordering::Relaxed);
        *counting = true;
    }

    if settled.0.load(Ordering::Relaxed) < SETTLED_FRAMES && !timed_out {
        return;
    }
    if timed_out {
        warn!("Showing the window before the UI finished loading");
    } else {
        info!("Showing the window after {:.0?}", time.elapsed());
    }
    window.visible = true;
}
