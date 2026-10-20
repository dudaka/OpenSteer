//! "Capture the Flag" (`plugins/CaptureTheFlag.cpp`): a seeker runs for the home base through
//! rocks while four enemies pursue it. The seeker avoids rocks first; otherwise it seeks home,
//! straight when the corridor to home is clear (`clearPathToGoal`) and mixed with a flee from
//! every enemy when not (`XXXsteerToEvadeAllDefenders`). After a tag or reaching home everyone
//! brakes, and 4 s later the round restarts from new random positions.

use std::f32::consts::TAU;

use steer_core::obstacle::{avoid_obstacles, Circle, SeenFrom};
use steer_core::util::limit_max_deviation_angle;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view, Trail};
use crate::controls::button;
use crate::view::FrameView;

const HOME: Vec2 = Vec2::ZERO;
const HOME_RADIUS: f32 = 1.5;
const BRAKING_RATE: f32 = 0.75;
const AVOID_TIME_MIN: f32 = 0.9;
const AVOID_TIME_MAX: f32 = 2.0;
const MAX_ROCKS: usize = 100;
const RESET_DELAY: f32 = 4.0;
const SEED: u64 = 1;
const SEEKER_BODY: u32 = 0x666699;
const ENEMY_BODY: u32 = 0x996666;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekerState {
    Running,
    Tagged,
    AtGoal,
}

#[derive(Debug, Clone)]
pub struct CaptureTheFlag {
    seeker: Vehicle,
    state: SeekerState,
    enemies: [Vehicle; 4],
    rocks: Vec<Circle>,
    rng: fastrand::Rng,
    trails: [Trail; 5],
    /// Time the seeker was last running, for the restart.
    last_running: f32,
    restarts: u32,
    time: f32,
}

impl Default for CaptureTheFlag {
    /// `CtfPlugIn::open`: 40 rocks, then the seeker and enemies on the 30-40 ring.
    fn default() -> Self {
        let mut demo = Self::from_layout(Vehicle::default(), Default::default(), Vec::new());
        demo.rng = fastrand::Rng::with_seed(SEED);
        for _ in 0..MAX_ROCKS * 2 / 5 {
            demo.add_rock();
        }
        demo.place_randomly();
        demo
    }
}

/// `CtfBase::reset`'s speeds: speed, max force and max speed 3.
fn ctf_vehicle(position: Vec2, forward: Vec2) -> Vehicle {
    Vehicle {
        position,
        forward,
        speed: 3.0,
        max_force: 3.0,
        max_speed: 3.0,
        ..Vehicle::default()
    }
}

/// `isAhead` / `isAside`: the cosine between `forward` and the direction to `target`.
fn cos_to(v: &Vehicle, target: Vec2) -> f32 {
    v.forward.dot((target - v.position).normalize())
}

impl CaptureTheFlag {
    /// A round from given positions and headings (speeds as `CtfBase::reset`).
    pub fn from_layout(seeker: Vehicle, enemies: [Vehicle; 4], rocks: Vec<Circle>) -> Self {
        let place = |v: Vehicle| ctf_vehicle(v.position, v.forward);
        Self {
            seeker: place(seeker),
            state: SeekerState::Running,
            enemies: enemies.map(place),
            rocks,
            rng: fastrand::Rng::with_seed(SEED),
            trails: Default::default(),
            last_running: 0.0,
            restarts: 0,
            time: 0.0,
        }
    }

    pub fn seeker(&self) -> &Vehicle {
        &self.seeker
    }

    pub fn enemies(&self) -> &[Vehicle; 4] {
        &self.enemies
    }

    pub fn state(&self) -> SeekerState {
        self.state
    }

    /// One step and the restart 4 s after the round ends (`updateState`).
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        if self.state == SeekerState::Running {
            self.last_running = self.time;
        } else if self.time > self.last_running + RESET_DELAY {
            self.place_randomly();
            self.restarts += 1;
            return;
        }
        self.step(dt);
        let all = std::iter::once(&self.seeker).chain(&self.enemies);
        for (trail, v) in self.trails.iter_mut().zip(all) {
            trail.record(self.time, v.position);
        }
    }

    /// `CtfPlugIn::update` without the restart: the seeker, then each enemy.
    pub fn step(&mut self, dt: f32) {
        if self.state == SeekerState::Running
            && (self.seeker.position - HOME).length() < self.seeker.radius + HOME_RADIUS
        {
            self.state = SeekerState::AtGoal;
        }
        let force = if self.state == SeekerState::Running {
            self.seeker_force()
        } else {
            self.seeker.apply_braking_force(BRAKING_RATE, dt);
            Vec2::ZERO
        };
        self.seeker.apply_steering_force(force, dt);
        for i in 0..self.enemies.len() {
            self.enemy_step(i, dt);
        }
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "add_rock" => self.add_rock(),
            "remove_rock" => {
                self.rocks.pop();
            }
            _ => {}
        }
    }

    pub fn view(&self) -> FrameView {
        let s = &self.seeker;
        let mut lines = circle(HOME, HOME_RADIUS, color::TARGET);
        lines.extend(
            self.rocks
                .iter()
                .flat_map(|r| circle(r.center, r.radius, color::OBSTACLE)),
        );
        if self.state == SeekerState::Running {
            let push = self.seeker_force();
            lines.push(line(s.position, s.position + push, color::PUSH));
        }
        let all = std::iter::once(s).chain(&self.enemies);
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 90.0,
            vehicles: std::iter::once(vehicle_view(s, SEEKER_BODY))
                .chain(self.enemies.iter().map(|e| vehicle_view(e, ENEMY_BODY)))
                .collect(),
            lines,
            trails: self
                .trails
                .iter()
                .zip(all)
                .map(|(t, v)| t.view(v.position))
                .collect(),
            labels: vec![
                label(s.position, self.seeker_status(), color::LABEL),
                label(
                    Vec2::new(0.0, -48.0),
                    format!("{} rocks, {} restarts", self.rocks.len(), self.restarts),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (color::TARGET, "home base: the seeker's goal"),
                (color::OBSTACLE, "rock"),
                (color::PUSH, "the seeker's push"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("add_rock", "Add rock", false),
                button("remove_rock", "Remove rock", false),
            ],
            params: Vec::new(),
        }
    }

    /// `CtfSeeker::steeringForSeeker`, which also sets the look-ahead.
    fn seeker_force(&self) -> Vec2 {
        let s = &self.seeker;
        let clear = self.clear_path_to_goal();
        let avoidance = avoid_obstacles(s, self.look_ahead(clear), &self.rocks);
        if avoidance != Vec2::ZERO {
            return avoidance;
        }
        let seek = steer::seek(s, HOME);
        if clear {
            limit_max_deviation_angle(seek, 0.707, s.forward)
        } else {
            limit_max_deviation_angle(seek + self.evade_all_defenders(), 0.707, s.forward)
        }
    }

    /// `adjustObstacleAvoidanceLookAhead`: look 2 s ahead when the way home is clear, the
    /// seeker heads straight for it and is not about to arrive; otherwise 0.9 s.
    fn look_ahead(&self, clear_path: bool) -> f32 {
        let s = &self.seeker;
        let heading_home = cos_to(s, HOME) > 0.98;
        let near = (HOME - s.position).length() / s.speed < AVOID_TIME_MAX;
        if clear_path && heading_home && !near {
            AVOID_TIME_MAX
        } else {
            AVOID_TIME_MIN
        }
    }

    /// `clearPathToGoal`: no enemy (predicted 0.3 x its travel time ahead) in the corridor
    /// to home, 8 radii wide, unless it is far enough behind.
    fn clear_path_to_goal(&self) -> bool {
        let s = &self.seeker;
        let side_threshold = s.radius * 8.0;
        let behind_threshold = s.radius * 2.0;
        let goal_offset = HOME - s.position;
        let goal_distance = goal_offset.length();
        let goal_direction = goal_offset / goal_distance;
        let cos_home = cos_to(s, HOME);
        let goal_is_aside = cos_home < 0.5 && cos_home > -0.5;
        self.enemies.iter().all(|e| {
            let distance = (s.position - e.position).length();
            let e_future = e.predict_future_position(0.3 * distance / e.speed);
            let e_offset = e_future - s.position;
            let along = goal_direction.dot(e_offset);
            let in_corridor = along > -behind_threshold && along < goal_distance;
            if !in_corridor || (e_offset - goal_direction * along).length() >= side_threshold {
                return true;
            }
            let e_front = s.forward.dot(e_offset) + e.radius;
            let behind = e_front < -behind_threshold;
            let way_behind = e_front < -2.0 * behind_threshold;
            (behind && goal_is_aside) || way_behind
        })
    }

    /// `XXXsteerToEvadeAllDefenders`: flee each enemy's position predicted 0.5 x its travel
    /// time ahead, weighted by 4 / distance, halved for enemies behind.
    fn evade_all_defenders(&self) -> Vec2 {
        let s = &self.seeker;
        self.enemies.iter().fold(Vec2::ZERO, |evade, e| {
            let offset = e.position - s.position;
            let distance = offset.length();
            let flee = steer::flee(s, e.predict_future_position(0.5 * distance / e.speed));
            let forward_weight = if s.forward.dot(offset) > s.radius * -2.0 {
                1.0
            } else {
                0.5
            };
            evade + flee * (4.0 / distance) * forward_weight
        })
    }

    /// `CtfEnemy::update`: avoid rocks, else pursue the seeker with a prediction capped at
    /// 0.9 x its time to home; brake once the round is over; tag on touch.
    fn enemy_step(&mut self, i: usize, dt: f32) {
        let s = &self.seeker;
        let e = &self.enemies[i];
        let to_goal = (HOME - s.position).length() - e.radius - HOME_RADIUS;
        let max_prediction = if to_goal < 0.0 {
            0.0
        } else {
            to_goal / s.speed
        } * 0.9;
        let force = if self.state == SeekerState::Running {
            let avoidance = avoid_obstacles(e, AVOID_TIME_MIN, &self.rocks);
            if avoidance == Vec2::ZERO {
                steer::pursuit(e, s, max_prediction)
            } else {
                avoidance
            }
        } else {
            self.enemies[i].apply_braking_force(BRAKING_RATE, dt);
            Vec2::ZERO
        };
        let e = &mut self.enemies[i];
        e.apply_steering_force(force, dt);
        let touching = (e.position - self.seeker.position).length() < e.radius + self.seeker.radius;
        if touching && self.state == SeekerState::Running {
            self.state = SeekerState::Tagged;
        }
    }

    fn seeker_status(&self) -> String {
        match self.state {
            SeekerState::Tagged => "tagged".into(),
            SeekerState::AtGoal => "reached goal".into(),
            SeekerState::Running => {
                let clear = self.clear_path_to_goal();
                let s = &self.seeker;
                if avoid_obstacles(s, self.look_ahead(clear), &self.rocks) != Vec2::ZERO {
                    "avoid obstacle".into()
                } else if clear {
                    "seek goal".into()
                } else {
                    "seek and evade".into()
                }
            }
        }
    }

    /// `randomizeStartingPositionAndHeading` for all five, and a running seeker.
    fn place_randomly(&mut self) {
        self.seeker = self.random_start();
        for i in 0..self.enemies.len() {
            self.enemies[i] = self.random_start();
        }
        self.state = SeekerState::Running;
        self.trails = Default::default();
        self.last_running = self.time;
    }

    /// On the 30-40 ring around home, at least 5 radii from any rock, random heading.
    fn random_start(&mut self) -> Vehicle {
        loop {
            let (angle, r) = (self.rng.f32() * TAU, 30.0 + 10.0 * self.rng.f32());
            let at = HOME + Vec2::new(angle.cos(), angle.sin()) * r;
            let v = ctf_vehicle(at, Vec2::new(0.0, 1.0));
            if self.clearance(at, 0.0) >= v.radius * 5.0 {
                let heading = self.rng.f32() * TAU;
                return ctf_vehicle(at, Vec2::new(heading.cos(), heading.sin()));
            }
        }
    }

    /// `addOneObstacle`: radius 1.5-4 within 44 of home, at least 2 clear of rocks and home.
    fn add_rock(&mut self) {
        if self.rocks.len() >= MAX_ROCKS {
            return;
        }
        let required = 0.5 * 4.0;
        loop {
            let r = 1.5 + 2.5 * self.rng.f32();
            let (angle, d) = (self.rng.f32() * TAU, self.rng.f32().sqrt() * 44.0);
            let c = Vec2::new(angle.cos(), angle.sin()) * d;
            let home_clearance = (c - HOME).length() - (r + HOME_RADIUS - required);
            if self.clearance(c, r).min(home_clearance) >= required {
                self.rocks.push(Circle::new(c, r, SeenFrom::Outside));
                return;
            }
        }
    }

    /// The smallest gap between a circle at `c` of radius `r` and any rock.
    fn clearance(&self, c: Vec2, r: f32) -> f32 {
        self.rocks
            .iter()
            .map(|o| (c - o.center).length() - (r + o.radius))
            .fold(f32::MAX, f32::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_round_has_forty_rocks_and_clear_starts() {
        let demo = CaptureTheFlag::default();
        assert_eq!(demo.rocks.len(), 40);
        let all = std::iter::once(&demo.seeker).chain(&demo.enemies);
        for v in all {
            let d = v.position.length();
            assert!((30.0..=40.0).contains(&d));
            assert!(demo.clearance(v.position, 0.0) >= 2.5);
        }
    }

    #[test]
    fn rounds_end_and_restart() {
        let mut demo = CaptureTheFlag::default();
        for _ in 0..60 * 120 {
            demo.update(1.0 / 60.0);
        }
        assert!(demo.restarts > 0);
    }

    #[test]
    fn rocks_can_be_added_and_removed() {
        let mut demo = CaptureTheFlag::default();
        demo.action("add_rock");
        assert_eq!(demo.rocks.len(), 41);
        demo.action("remove_rock");
        demo.action("remove_rock");
        assert_eq!(demo.rocks.len(), 39);
    }
}
