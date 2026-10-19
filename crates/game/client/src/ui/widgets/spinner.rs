use crate::ui::theme::palette;
use bevy::prelude::*;

const TRACK_WIDTH: f32 = 160.0;
const BLOCK_WIDTH: f32 = 40.0;
/// Seconds for the block to cross the track and back.
const PERIOD: f32 = 1.6;

/// The sliding block of a [`spinner`].
#[derive(Component, Default, Clone)]
pub struct SpinnerBlock;

/// An indeterminate progress bar: an accent block sliding back and forth along a thin track.
pub fn spinner() -> impl Scene {
    bsn! {
        Node { width: px(TRACK_WIDTH), height: px(4) }
        BackgroundColor(palette::PANEL_BORDER)
        Children [(
            SpinnerBlock
            Node { position_type: PositionType::Absolute, width: px(BLOCK_WIDTH), height: percent(100) }
            BackgroundColor(palette::ACCENT)
        )]
    }
}

pub(super) fn animate_spinners(time: Res<Time>, mut blocks: Query<&mut Node, With<SpinnerBlock>>) {
    // eased ping-pong between the track ends
    let phase = (time.elapsed_secs() / PERIOD).fract();
    let t = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    for mut node in &mut blocks {
        node.left = px(t * (TRACK_WIDTH - BLOCK_WIDTH));
    }
}
