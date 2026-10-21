//! "3D Pursuit and Evasion": the 2D demo in space. Multiple pursuit: a 3D wanderer (forward +
//! 3 x wander) and pursuers that start on a shell 20-30 units around it, with random 3D
//! headings, and restart there when they touch it. Pursuit vs evasion: one hunter from
//! (-5, -2, -5) and an evader (max speed 2), as `oracle/pursuit_3d.cpp` until the first catch.

use std::f32::consts::TAU;

use steer_core_3d::steer::{self, WanderState};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend, ParamView};

use crate::annotation::{label, line, point, stem, vehicle_view, Trail};
use crate::view::{Frame3View, Label3View, Line3View};

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
pub struct Pursuit3 {
    mode: Mode,
    quarry: Agent,
    walk: WanderState,
    pursuers: Vec<Agent>,
    pursuer_count: usize,
    rng: fastrand::Rng,
    /// Pursuers that touched the quarry, since the start.
    catches: u32,
    time: f32,
}

impl Default for Pursuit3 {
    fn default() -> Self {
        Self::new(Mode::Multiple, 30)
    }
}

/// `MpBase::reset`: at rest, max_force 5, max_speed 3.
fn mp_vehicle(position: Vec3) -> Agent {
    Agent {
        vehicle: Vehicle {
            position,
            max_force: 5.0,
            max_speed: 3.0,
            ..Vehicle::default()
        },
        trail: Trail::default(),
    }
}

/// A uniformly random unit vector (Archimedes: uniform height, uniform angle).
fn random_direction(rng: &mut fastrand::Rng) -> Vec3 {
    let y = rng.f32() * 2.0 - 1.0;
    let angle = rng.f32() * TAU;
    let r = (1.0 - y * y).sqrt();
    Vec3::new(r * angle.cos(), y, r * angle.sin())
}

impl Pursuit3 {
    fn new(mode: Mode, pursuer_count: usize) -> Self {
        let mut demo = Self {
            mode,
            quarry: mp_vehicle(Vec3::ZERO),
            walk: WanderState::default(),
            pursuers: Vec::new(),
            pursuer_count,
            rng: fastrand::Rng::with_seed(SEED),
            catches: 0,
            time: 0.0,
        };
        match mode {
            Mode::Multiple => {
                for _ in 0..pursuer_count {
                    let pursuer = demo.on_shell();
                    demo.pursuers.push(pursuer);
                }
            }
            Mode::Evasion => {
                demo.quarry.vehicle.max_speed = 2.0;
                demo.pursuers.push(mp_vehicle(Vec3::new(-5.0, -2.0, -5.0)));
            }
        }
        demo
    }

    /// `randomizeStartingPositionAndHeading` in 3D: 20-30 units from the quarry in a random
    /// direction, facing a random direction.
    fn on_shell(&mut self) -> Agent {
        let direction = random_direction(&mut self.rng);
        let radius = 20.0 + 10.0 * self.rng.f32();
        let mut agent = mp_vehicle(self.quarry.vehicle.position + direction * radius);
        agent
            .vehicle
            .regenerate_basis(random_direction(&mut self.rng));
        agent
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = match self.mode {
            Mode::Multiple => {
                let rng = &mut self.rng;
                let q = &self.quarry.vehicle;
                let wander = steer::wander(q, &mut self.walk, dt, &mut || rng.f32());
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
                self.catches += 1;
                self.pursuers[i] = self.on_shell();
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

    pub fn view(&self) -> Frame3View {
        let q = &self.quarry.vehicle;
        let mut lines: Vec<Line3View> = self
            .pursuers
            .iter()
            .map(|p| {
                let aim = steer::intercept(&p.vehicle, q, PURSUIT_MAX_TIME).target;
                line(p.vehicle.position, aim, color::TARGET)
            })
            .collect();
        lines.push(stem(q.position));
        lines.extend(self.pursuers.iter().map(|p| stem(p.vehicle.position)));
        let quarry_name = match self.mode {
            Mode::Multiple => "quarry (wanders)",
            Mode::Evasion => "quarry (evades)",
        };
        let mut labels = vec![
            label(q.position, quarry_name, color::LABEL),
            label(
                q.position + Vec3::new(0.0, -3.0, 0.0),
                format!("catches: {}", self.catches),
                color::LABEL,
            ),
        ];
        labels.extend(self.nearest_label());
        Frame3View {
            time: self.time,
            focus: point(q.position),
            view_size: 70.0,
            vehicles: [vehicle_view(q, QUARRY_BODY)]
                .into_iter()
                .chain(
                    self.pursuers
                        .iter()
                        .map(|p| vehicle_view(&p.vehicle, PURSUER_BODY)),
                )
                .collect(),
            lines,
            trails: [&self.quarry]
                .into_iter()
                .chain(&self.pursuers)
                .map(|a| a.trail.view(a.vehicle.position))
                .collect(),
            labels,
            legend: legend(&[
                (color::TARGET, "aim: the quarry's predicted position"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
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
    fn nearest_label(&self) -> Option<Label3View> {
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

    fn run(demo: &mut Pursuit3, steps: u32) {
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
        }
    }

    #[test]
    fn pursuers_start_on_the_shell_in_all_directions() {
        let demo = Pursuit3::default();
        assert_eq!(demo.pursuers.len(), 30);
        for p in &demo.pursuers {
            let d = p.vehicle.position.length();
            assert!((20.0..=30.0).contains(&d), "{d}");
        }
        assert!(demo.pursuers.iter().any(|p| p.vehicle.position.y > 10.0));
        assert!(demo.pursuers.iter().any(|p| p.vehicle.position.y < -10.0));
    }

    /// Pursuers chasing the 3D wanderer catch it 20 times in a minute.
    #[test]
    fn pursuers_catch_the_wanderer_and_restart() {
        let mut demo = Pursuit3::default();
        run(&mut demo, 60 * 60);
        assert_eq!(demo.catches, 20);
    }

    /// Pursuit vs evasion is the oracle's run (`oracle_pursuit_3d`, evader at step 180) until
    /// the hunter catches the evader at step 232 (plain flee slows near the menace) and
    /// restarts on the shell.
    #[test]
    fn evasion_runs_the_oracle_scenario_until_the_catch() {
        let mut demo = Pursuit3::default();
        demo.action("evasion");
        run(&mut demo, 180);
        let p = demo.quarry.vehicle.position;
        assert!(
            (p - Vec3::new(2.788961, 1.115584, 1.667931)).length() < 1e-4,
            "{p:?}"
        );
        run(&mut demo, 51);
        assert_eq!(demo.catches, 0);
        run(&mut demo, 1);
        assert_eq!(demo.catches, 1);
    }
}
