//! The simulation: one demo advanced in fixed steps, controlled by commands from clients.
//! The sim task owns all state; clients reach it only through the command channel.

use std::time::Duration;

use steer_demos::OneTurning;
use tokio::sync::{mpsc, watch};
use tokio::time::{interval, MissedTickBehavior};

use crate::contract::HostFrame;

/// Fixed simulation step, in seconds (60 Hz).
pub const STEP: f32 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Play,
    Pause,
    Step,
    Reset,
}

/// The demo and whether time is running.
#[derive(Debug)]
pub struct Sim {
    demo: OneTurning,
    running: bool,
}

impl Default for Sim {
    fn default() -> Self {
        Self {
            demo: OneTurning::default(),
            running: true,
        }
    }
}

impl Sim {
    /// Applies the commands received since the last tick, then advances one step if running.
    /// Returns whether the world or the running state changed.
    pub fn tick(&mut self, commands: impl IntoIterator<Item = Command>) -> bool {
        let mut changed = self.running;
        for command in commands {
            changed = true;
            match command {
                Command::Play => self.running = true,
                Command::Pause => self.running = false,
                Command::Step => {
                    self.running = false;
                    self.demo.update(STEP);
                }
                Command::Reset => self.demo = OneTurning::default(),
            }
        }
        if self.running {
            self.demo.update(STEP);
        }
        changed
    }

    pub fn frame(&self) -> HostFrame {
        HostFrame::Frame {
            frame: self.demo.view(),
            running: self.running,
        }
    }
}

/// Runs forever, publishing the latest frame whenever it changes.
pub async fn run(
    mut sim: Sim,
    frames: watch::Sender<HostFrame>,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut tick = interval(Duration::from_secs_f32(STEP));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        if sim.tick(std::iter::from_fn(|| commands.try_recv().ok())) {
            frames.send_replace(sim.frame());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ticks once; the frame's time and running state if anything changed.
    fn tick(sim: &mut Sim, commands: &[Command]) -> Option<(f32, bool)> {
        sim.tick(commands.iter().copied())
            .then(|| match sim.frame() {
                HostFrame::Frame { frame, running } => (frame.time, running),
                other => panic!("not a frame: {other:?}"),
            })
    }

    #[test]
    fn running_advances_every_tick() {
        let mut sim = Sim::default();
        assert_eq!(tick(&mut sim, &[]), Some((STEP, true)));
        assert_eq!(tick(&mut sim, &[]), Some((2.0 * STEP, true)));
    }

    #[test]
    fn paused_changes_once_then_nothing() {
        let mut sim = Sim::default();
        assert_eq!(tick(&mut sim, &[Command::Pause]), Some((0.0, false)));
        assert_eq!(tick(&mut sim, &[]), None);
    }

    #[test]
    fn step_pauses_and_advances_exactly_once() {
        let mut sim = Sim::default();
        tick(&mut sim, &[]);
        assert_eq!(tick(&mut sim, &[Command::Step]), Some((2.0 * STEP, false)));
        assert_eq!(tick(&mut sim, &[]), None);
        assert_eq!(tick(&mut sim, &[Command::Step]), Some((3.0 * STEP, false)));
    }

    #[test]
    fn reset_restarts_the_demo_and_keeps_running_state() {
        let mut sim = Sim::default();
        tick(&mut sim, &[Command::Step]);
        assert_eq!(tick(&mut sim, &[Command::Reset]), Some((0.0, false)));
        assert_eq!(
            tick(&mut sim, &[Command::Play, Command::Reset]),
            Some((STEP, true))
        );
    }
}
