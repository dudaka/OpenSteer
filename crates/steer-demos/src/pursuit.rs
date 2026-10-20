//! "Pursuit and Evasion". Multiple pursuit (`plugins/MultiplePursuit.cpp`): one wanderer and
//! pursuers that start on a ring 20-30 units out and restart there when they touch it. Each
//! pursuer aims at `steerForPursuit`'s predicted intercept. The second mode is one hunter
//! pursuing an evader (`steerForEvasion`, max speed 2).

use std::f32::consts::TAU;

use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{color, label, legend, line, point, vehicle_view, Trail};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LabelView, LineView, ParamView};

const PURSUERS: Range = Range::new(1.0, 30.0, 1.0);
const SEED: u64 = 1;
/// `MpPursuer::update`'s maxTime.
const PURSUIT_MAX_TIME: f32 = 20.0;
const EVASION_MAX_TIME: f32 = 2.0;
/// Body colors: `MpWanderer` greenish, `MpPursuer` reddish.
const QUARRY_BODY: u32 = 0x669966;
const PURSUER_BODY: u32 = 0x996666;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Multiple,
    Evasion,
}

#[derive(Debug, Clone)]
struct Agent {
    vehicle: Vehicle,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Pursuit {
    mode: Mode,
    quarry: Agent,
    wander_side: f32,
    pursuers: Vec<Agent>,
    pursuer_count: usize,
    rng: fastrand::Rng,
    time: f32,
}

impl Default for Pursuit {
    fn default() -> Self {
        Self::new(Mode::Multiple, 30)
    }
}

/// `MpBase::reset`: at rest, max_force 5, max_speed 3.
fn mp_vehicle(position: Vec2, forward: Vec2) -> Agent {
    Agent {
        vehicle: Vehicle {
            position,
            forward,
            max_force: 5.0,
            max_speed: 3.0,
            ..Vehicle::default()
        },
        trail: Trail::default(),
    }
}

impl Pursuit {
    fn new(mode: Mode, pursuer_count: usize) -> Self {
        let mut demo = Self {
            mode,
            quarry: mp_vehicle(Vec2::ZERO, Vec2::new(0.0, 1.0)),
            wander_side: 0.0,
            pursuers: Vec::new(),
            pursuer_count,
            rng: fastrand::Rng::with_seed(SEED),
            time: 0.0,
        };
        match mode {
            Mode::Multiple => {
                for _ in 0..pursuer_count {
                    let pursuer = demo.on_ring();
                    demo.pursuers.push(pursuer);
                }
            }
            Mode::Evasion => {
                demo.quarry.vehicle.max_speed = 2.0;
                demo.pursuers
                    .push(mp_vehicle(Vec2::new(-5.0, -5.0), Vec2::new(0.0, 1.0)));
            }
        }
        demo
    }

    /// `randomizeStartingPositionAndHeading`: on a ring 20-30 units around the quarry, random
    /// heading.
    fn on_ring(&mut self) -> Agent {
        let angle = self.rng.f32() * TAU;
        let radius = 20.0 + 10.0 * self.rng.f32();
        let heading = self.rng.f32() * TAU;
        mp_vehicle(
            self.quarry.vehicle.position + Vec2::new(angle.cos(), angle.sin()) * radius,
            Vec2::new(heading.cos(), heading.sin()),
        )
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = match self.mode {
            Mode::Multiple => {
                let rng = &mut self.rng;
                let q = &self.quarry.vehicle;
                let wander = steer::wander(q, &mut self.wander_side, dt, &mut || rng.f32());
                q.forward + wander * 3.0
            }
            Mode::Evasion => steer::evasion(
                &self.quarry.vehicle,
                &self.pursuers[0].vehicle,
                EVASION_MAX_TIME,
            ),
        };
        self.quarry.vehicle.apply_steering_force(force, dt);
        self.quarry
            .trail
            .record(self.time, self.quarry.vehicle.position);
        for i in 0..self.pursuers.len() {
            if self.touches_quarry(&self.pursuers[i].vehicle) {
                self.pursuers[i] = self.on_ring();
            }
            let p = &mut self.pursuers[i];
            let force = steer::pursuit(&p.vehicle, &self.quarry.vehicle, PURSUIT_MAX_TIME);
            p.vehicle.apply_steering_force(force, dt);
            p.trail.record(self.time, p.vehicle.position);
        }
    }

    fn touches_quarry(&self, v: &Vehicle) -> bool {
        let q = &self.quarry.vehicle;
        (v.position - q.position).length() < v.radius + q.radius
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "multiple" => *self = Self::new(Mode::Multiple, self.pursuer_count),
            "evasion" => *self = Self::new(Mode::Evasion, self.pursuer_count),
            _ => {}
        }
    }

    /// The pursuer count restarts the demo with that many pursuers.
    pub fn param(&mut self, id: &str, value: f32) {
        if id == "pursuers" {
            *self = Self::new(self.mode, PURSUERS.clamp(value) as usize);
        }
    }

    pub fn view(&self) -> FrameView {
        let q = &self.quarry.vehicle;
        let aims: Vec<LineView> = self
            .pursuers
            .iter()
            .map(|p| {
                let aim = steer::intercept(&p.vehicle, q, PURSUIT_MAX_TIME).target;
                line(p.vehicle.position, aim, color::TARGET)
            })
            .collect();
        let quarry_name = match self.mode {
            Mode::Multiple => "quarry (wanders)",
            Mode::Evasion => "quarry (evades)",
        };
        let mut labels = vec![label(q.position, quarry_name, color::LABEL)];
        labels.extend(self.nearest_label());
        FrameView {
            time: self.time,
            focus: point(q.position),
            view_size: 60.0,
            vehicles: [vehicle_view(q, QUARRY_BODY)]
                .into_iter()
                .chain(
                    self.pursuers
                        .iter()
                        .map(|p| vehicle_view(&p.vehicle, PURSUER_BODY)),
                )
                .collect(),
            lines: aims,
            trails: [&self.quarry]
                .into_iter()
                .chain(&self.pursuers)
                .map(|a| a.trail.view(a.vehicle.position))
                .collect(),
            labels,
            legend: legend(&[
                (color::TARGET, "aim: the quarry's predicted position"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("multiple", "Multiple pursuit", self.mode == Mode::Multiple),
                button("evasion", "Pursuit vs evasion", self.mode == Mode::Evasion),
            ],
            params: self.params(),
        }
    }

    fn params(&self) -> Vec<ParamView> {
        match self.mode {
            Mode::Multiple => vec![slider(
                "pursuers",
                "pursuers",
                PURSUERS,
                self.pursuer_count as f32,
            )],
            Mode::Evasion => Vec::new(),
        }
    }

    /// The nearest pursuer's table case and time factor.
    fn nearest_label(&self) -> Option<LabelView> {
        let q = &self.quarry.vehicle;
        let nearest = self.pursuers.iter().min_by(|a, b| {
            let da = (a.vehicle.position - q.position).length();
            let db = (b.vehicle.position - q.position).length();
            da.total_cmp(&db)
        })?;
        let i = steer::intercept(&nearest.vehicle, q, PURSUIT_MAX_TIME);
        Some(label(
            nearest.vehicle.position,
            format!("{}: x{}", i.case, i.time_factor),
            color::LABEL,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut Pursuit, steps: u32) {
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
        }
    }

    #[test]
    fn pursuers_start_on_the_ring() {
        let demo = Pursuit::default();
        assert_eq!(demo.pursuers.len(), 30);
        for p in &demo.pursuers {
            let d = p.vehicle.position.length();
            assert!((20.0..=30.0).contains(&d), "{d}");
        }
    }

    #[test]
    fn pursuers_close_in_and_restart_on_touch() {
        let mut demo = Pursuit::default();
        let mut restarts = 0;
        for _ in 0..60 * 60 {
            let before: Vec<Vec2> = demo.pursuers.iter().map(|p| p.vehicle.position).collect();
            demo.update(1.0 / 60.0);
            restarts += demo
                .pursuers
                .iter()
                .zip(before)
                .filter(|(p, b)| (p.vehicle.position - *b).length() > 5.0)
                .count();
        }
        assert!(restarts > 0);
    }

    #[test]
    fn evasion_mode_has_one_hunter_and_a_slower_quarry() {
        let mut demo = Pursuit::default();
        demo.action("evasion");
        run(&mut demo, 600);
        assert_eq!(demo.pursuers.len(), 1);
        assert!(demo.quarry.vehicle.speed <= 2.0 + 1e-5);
        assert!(demo.view().params.is_empty());
    }

    #[test]
    fn pursuer_slider_restarts_with_that_many() {
        let mut demo = Pursuit::default();
        demo.param("pursuers", 5.0);
        assert_eq!(demo.pursuers.len(), 5);
        assert_eq!(demo.time, 0.0);
    }
}
