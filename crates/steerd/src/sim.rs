//! The simulation loop: advances the world in fixed steps and broadcasts each frame.

use std::time::Duration;

use steer_demos::OneTurning;
use tokio::sync::broadcast;
use tokio::time::{interval, MissedTickBehavior};

use crate::contract::HostFrame;

/// Fixed simulation step, in seconds (60 Hz).
pub const STEP: f32 = 1.0 / 60.0;

/// Runs forever; a send with no client connected is simply dropped.
pub async fn run(frames: broadcast::Sender<HostFrame>) {
    let mut world = OneTurning::default();
    let mut tick = interval(Duration::from_secs_f32(STEP));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        world.update(STEP);
        let _ = frames.send(HostFrame::Frame {
            frame: world.view(),
        });
    }
}
