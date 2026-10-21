//! "Soccer" (`plugins/Soccer.cpp`, "Michael's Simple Soccer"): two teams of 8 and a ball. A
//! player touching the ball kicks it away from itself; otherwise it avoids neighbors (1 s),
//! and when the ball is within 12 of its home spot it either seeks the ball (when it is on the
//! right side of it) or gets behind it, else it goes home. The ball brakes, bounces off the
//! field edges and restarts at the center after a goal. Each player avoids only the players
//! created before it: the plug-in copies the player list into each player as it is built.

use steer_core::neighbors::avoid_neighbors;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{color, label, legend, line, vehicle_view, Trail};
use crate::view::{FrameView, LineView};

/// `playerPosition`: team A's spots; team B mirrors x.
const SPOTS: [(f32, f32); 8] = [
    (4.0, 0.0),
    (7.0, -5.0),
    (7.0, 5.0),
    (10.0, -3.0),
    (10.0, 3.0),
    (15.0, -8.0),
    (15.0, 0.0),
    (15.0, 8.0),
];
const FIELD: (Vec2, Vec2) = (Vec2::new(-20.0, -10.0), Vec2::new(20.0, 10.0));
/// Ball here scores for red (team A attacks -x) / for blue.
const GOAL_A: (Vec2, Vec2) = (Vec2::new(-21.0, -7.0), Vec2::new(-19.0, 7.0));
const GOAL_B: (Vec2, Vec2) = (Vec2::new(19.0, -7.0), Vec2::new(21.0, 7.0));
const RED_BODY: u32 = 0xb35555;
const BLUE_BODY: u32 = 0x5566b3;
const BALL_BODY: u32 = 0xe8e8e8;

#[derive(Debug, Clone)]
struct Player {
    vehicle: Vehicle,
    team_a: bool,
    home: Vec2,
}

#[derive(Debug, Clone)]
pub struct Soccer {
    players: Vec<Player>,
    ball: Vehicle,
    ball_trail: Trail,
    red: u32,
    blue: u32,
    time: f32,
}

impl Default for Soccer {
    /// `MicTestPlugIn::open`: team A (red) then team B (blue), each on its spots.
    fn default() -> Self {
        let players = [true, false]
            .into_iter()
            .flat_map(|team_a| {
                SPOTS.iter().map(move |&(x, y)| {
                    let home = Vec2::new(if team_a { x } else { -x }, y);
                    Player {
                        vehicle: Vehicle {
                            position: home,
                            max_force: 3000.7,
                            max_speed: 10.0,
                            ..Vehicle::default()
                        },
                        team_a,
                        home,
                    }
                })
            })
            .collect();
        Self {
            players,
            ball: new_ball(),
            ball_trail: Trail::default(),
            red: 0,
            blue: 0,
            time: 0.0,
        }
    }
}

/// `Ball::reset`: at rest at the center, max force and speed 9.
fn new_ball() -> Vehicle {
    Vehicle {
        max_force: 9.0,
        max_speed: 9.0,
        ..Vehicle::default()
    }
}

fn inside(p: Vec2, (min, max): (Vec2, Vec2)) -> (bool, bool) {
    (p.x >= min.x && p.x <= max.x, p.y >= min.y && p.y <= max.y)
}

impl Soccer {
    pub fn ball(&self) -> &Vehicle {
        &self.ball
    }

    pub fn player(&self, i: usize) -> &Vehicle {
        &self.players[i].vehicle
    }

    /// (red, blue).
    pub fn score(&self) -> (u32, u32) {
        (self.red, self.blue)
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.step(dt);
        self.ball_trail.record(self.time, self.ball.position);
    }

    /// `MicTestPlugIn::update`: every player, then the ball, then the goals.
    pub fn step(&mut self, dt: f32) {
        for i in 0..self.players.len() {
            self.player_step(i, dt);
        }
        self.ball_step(dt);
        let p = self.ball.position;
        if inside(p, GOAL_A) == (true, true) {
            self.ball = new_ball();
            self.red += 1;
        }
        if inside(p, GOAL_B) == (true, true) {
            self.ball = new_ball();
            self.blue += 1;
        }
    }

    pub fn view(&self) -> FrameView {
        let (min, max) = FIELD;
        let mut lines = rect(min, max, color::EDGE);
        lines.extend(rect(GOAL_A.0, GOAL_A.1, color::TARGET));
        lines.extend(rect(GOAL_B.0, GOAL_B.1, color::TARGET));
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 26.0,
            vehicles: self
                .players
                .iter()
                .map(|p| vehicle_view(&p.vehicle, if p.team_a { RED_BODY } else { BLUE_BODY }))
                .chain([vehicle_view(&self.ball, BALL_BODY)])
                .collect(),
            lines,
            trails: vec![self.ball_trail.view(self.ball.position)],
            labels: vec![label(
                Vec2::new(0.0, -11.5),
                format!("red {} : {} blue", self.red, self.blue),
                color::LABEL,
            )],
            legend: legend(&[
                (color::EDGE, "field edge: the ball bounces"),
                (color::TARGET, "goals: red scores on the left"),
                (color::TRAIL, "ball trail: the last 5 s"),
            ]),
            actions: Vec::new(),
            params: Vec::new(),
        }
    }

    /// `Player::update`.
    fn player_step(&mut self, i: usize, dt: f32) {
        let ball = self.ball.position;
        let p = &self.players[i];
        let v = &p.vehicle;
        if (v.position - ball).length() < v.radius + self.ball.radius {
            let kick = (ball - v.position) * 50.0;
            self.ball.speed = kick.length();
            self.ball.regenerate_basis(kick.normalize());
        }
        let earlier: Vec<&Vehicle> = self.players[..i].iter().map(|o| &o.vehicle).collect();
        let avoidance = avoid_neighbors(v, 1.0, &earlier);
        let force = if avoidance != Vec2::ZERO {
            avoidance
        } else if (p.home - ball).length() < 12.0 {
            let on_ball_side = if p.team_a {
                v.position.x > ball.x
            } else {
                v.position.x < ball.x
            };
            if on_ball_side {
                steer::seek(v, ball)
            } else {
                let z = if ball.y - v.position.y > 0.0 {
                    -1.0
                } else {
                    1.0
                };
                let behind = ball + Vec2::new(if p.team_a { 2.0 } else { -2.0 }, z);
                steer::seek(v, behind) * 10.0 + steer::flee(v, ball)
            }
        } else {
            steer::seek(v, p.home) + steer::seek(v, p.home)
        };
        self.players[i].vehicle.apply_steering_force(force, dt);
    }

    /// `Ball::update`: brake, roll on, and bounce off the field edges.
    fn ball_step(&mut self, dt: f32) {
        let b = &mut self.ball;
        b.apply_braking_force(1.5, dt);
        b.apply_steering_force(b.velocity(), dt);
        let (in_x, _) = inside(b.position, FIELD);
        if !in_x {
            let d = b.velocity();
            b.regenerate_basis(Vec2::new(-d.x, d.y).normalize());
            b.apply_steering_force(b.velocity(), dt);
        }
        let (_, in_y) = inside(b.position, FIELD);
        if !in_y {
            let d = b.velocity();
            b.regenerate_basis(Vec2::new(d.x, -d.y).normalize());
            b.apply_steering_force(b.velocity(), dt);
        }
    }
}

fn rect(min: Vec2, max: Vec2, color: u32) -> Vec<LineView> {
    let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
    (0..4)
        .map(|i| line(corners[i], corners[(i + 1) % 4], color))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teams_mirror_each_other() {
        let demo = Soccer::default();
        assert_eq!(demo.players.len(), 16);
        assert_eq!(demo.players[8].home, Vec2::new(-4.0, 0.0));
    }

    #[test]
    fn the_ball_stays_on_the_field() {
        let mut demo = Soccer::default();
        for _ in 0..60 * 120 {
            demo.update(1.0 / 60.0);
            let p = demo.ball.position;
            assert!(p.x.abs() < 21.0 && p.y.abs() < 11.0, "{p:?}");
        }
    }
}
