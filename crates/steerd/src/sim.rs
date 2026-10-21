//! The simulation: one demo advanced in fixed steps, controlled by commands from clients.
//! The sim task owns all state; clients reach it only through the command channel.

use std::time::Duration;

use steer_core::Vec2;
use steer_demos::{Demo, DemoInfo, DEFAULT_DEMO};
use steer_demos_3d::{Demo3, TerrainView};
use tokio::sync::{mpsc, watch};
use tokio::time::{interval, MissedTickBehavior};

use crate::contract::{HostFrame, SceneFrame};

/// Fixed simulation step, in seconds (60 Hz).
pub const STEP: f32 = 1.0 / 60.0;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Play,
    Pause,
    Step,
    Reset,
    /// A demo id already checked against the catalog.
    Select(String),
    Click(Vec2),
    Action(String),
    Param(String, f32),
}

/// Every demo the host offers: the 2D ones, then the 3D ones.
pub fn catalog() -> Vec<DemoInfo> {
    steer_demos::catalog()
        .into_iter()
        .chain(steer_demos_3d::catalog())
        .collect()
}

/// A 2D or a 3D demo.
#[derive(Debug)]
enum AnyDemo {
    Flat(Demo),
    Space(Demo3),
}

impl AnyDemo {
    fn new(id: &str) -> Option<Self> {
        Demo::new(id)
            .map(AnyDemo::Flat)
            .or_else(|| Demo3::new(id).map(AnyDemo::Space))
    }

    fn update(&mut self, dt: f32) {
        match self {
            AnyDemo::Flat(d) => d.update(dt),
            AnyDemo::Space(d) => d.update(dt),
        }
    }

    /// 3D demos take no clicks.
    fn click(&mut self, at: Vec2) {
        if let AnyDemo::Flat(d) = self {
            d.click(at);
        }
    }

    fn action(&mut self, id: &str) {
        match self {
            AnyDemo::Flat(d) => d.action(id),
            AnyDemo::Space(d) => d.action(id),
        }
    }

    fn param(&mut self, id: &str, value: f32) {
        match self {
            AnyDemo::Flat(d) => d.param(id, value),
            AnyDemo::Space(d) => d.param(id, value),
        }
    }

    fn terrain(&self) -> Option<TerrainView> {
        match self {
            AnyDemo::Flat(_) => None,
            AnyDemo::Space(d) => d.terrain(),
        }
    }

    fn view(&self) -> SceneFrame {
        match self {
            AnyDemo::Flat(d) => SceneFrame::Flat(d.view()),
            AnyDemo::Space(d) => SceneFrame::Space(d.view()),
        }
    }
}

/// The current demo and whether time is running.
#[derive(Debug)]
pub struct Sim {
    id: String,
    demo: AnyDemo,
    running: bool,
}

impl Default for Sim {
    fn default() -> Self {
        Self {
            id: DEFAULT_DEMO.to_string(),
            demo: AnyDemo::new(DEFAULT_DEMO).expect("the default demo is in the catalog"),
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
                Command::Reset => self.demo = Self::fresh(&self.id),
                Command::Select(id) => {
                    self.demo = Self::fresh(&id);
                    self.id = id;
                }
                Command::Click(at) => self.demo.click(at),
                Command::Action(id) => self.demo.action(&id),
                Command::Param(id, value) => self.demo.param(&id, value),
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
            demo: self.id.clone(),
        }
    }

    /// The current demo's ground.
    pub fn terrain(&self) -> HostFrame {
        HostFrame::Terrain {
            demo: self.id.clone(),
            terrain: self.demo.terrain(),
        }
    }

    fn fresh(id: &str) -> AnyDemo {
        AnyDemo::new(id).expect("demo ids are checked against the catalog before they get here")
    }
}

/// Runs forever, publishing the latest frame whenever it changes, and the demo's ground whenever
/// another demo is chosen.
pub async fn run(
    mut sim: Sim,
    frames: watch::Sender<HostFrame>,
    terrain: watch::Sender<HostFrame>,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut tick = interval(Duration::from_secs_f32(STEP));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        let shown = sim.id.clone();
        if sim.tick(std::iter::from_fn(|| commands.try_recv().ok())) {
            if sim.id != shown {
                terrain.send_replace(sim.terrain());
            }
            frames.send_replace(sim.frame());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ticks once; the frame's time and running state if anything changed.
    fn tick(sim: &mut Sim, commands: &[Command]) -> Option<(f32, bool)> {
        sim.tick(commands.iter().cloned())
            .then(|| match sim.frame() {
                HostFrame::Frame {
                    frame: SceneFrame::Flat(frame),
                    running,
                    ..
                } => (frame.time, running),
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

    #[test]
    fn a_3d_demo_sends_space_frames_and_ignores_clicks() {
        let mut sim = Sim::default();
        sim.tick([Command::Select("3d-seek-flee".into())]);
        sim.tick([Command::Click(Vec2::new(1.0, 2.0))]);
        match sim.frame() {
            HostFrame::Frame {
                demo,
                frame: SceneFrame::Space(frame),
                ..
            } => {
                assert_eq!(demo, "3d-seek-flee");
                assert_eq!(frame.time, 2.0 * STEP);
            }
            other => panic!("not a 3D frame: {other:?}"),
        }
    }

    /// The ground message names the demo; a 2D demo has none. (The Silkroad demo's ground
    /// needs the game data; its content is tested in `steer-demos-3d`.)
    #[test]
    fn the_terrain_frame_names_the_demo() {
        let mut sim = Sim::default();
        assert_eq!(
            sim.terrain(),
            HostFrame::Terrain {
                demo: DEFAULT_DEMO.into(),
                terrain: None
            }
        );
        sim.tick([Command::Select("3d-silkroad-seek".into())]);
        match sim.terrain() {
            HostFrame::Terrain { demo, .. } => assert_eq!(demo, "3d-silkroad-seek"),
            other => panic!("not a terrain frame: {other:?}"),
        }
    }

    #[test]
    fn select_switches_demo_and_reset_restarts_the_selected_one() {
        let mut sim = Sim::default();
        tick(&mut sim, &[Command::Select("seek-flee".into())]);
        tick(&mut sim, &[Command::Reset]);
        match sim.frame() {
            HostFrame::Frame {
                demo,
                frame: SceneFrame::Flat(frame),
                ..
            } => {
                assert_eq!(demo, "seek-flee");
                assert_eq!(frame.actions.len(), 4);
            }
            other => panic!("not a frame: {other:?}"),
        }
    }
}
