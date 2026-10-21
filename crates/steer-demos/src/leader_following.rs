//! "Leader Following" (Reynolds 1999, not in OpenSteer, so no C++ oracle): a wandering leader
//! (the Wander demo's push) and followers using `steer_core::leader::follow_leader`.

use steer_core::leader::{behind_point, follow_leader, FollowCase, LeaderFollow};
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, point, vehicle_view, Trail};
use crate::controls::{slider, Range};
use crate::view::{FrameView, LineView};

const FOLLOWERS: Range = Range::new(1.0, 20.0, 1.0);
const BEHIND: Range = Range::new(1.0, 8.0, 0.5);
const SEPARATION: Range = Range::new(0.0, 5.0, 0.25);
const SEED: u64 = 1;

#[derive(Debug, Clone)]
struct Follower {
    vehicle: Vehicle,
    case: FollowCase,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct LeaderFollowing {
    leader: Vehicle,
    wander_side: f32,
    rng: fastrand::Rng,
    followers: Vec<Follower>,
    settings: LeaderFollow,
    leader_trail: Trail,
    time: f32,
}

impl Default for LeaderFollowing {
    fn default() -> Self {
        Self::new(8, settings(3.0, 2.0))
    }
}

fn settings(behind: f32, separation_weight: f32) -> LeaderFollow {
    LeaderFollow {
        behind,
        slowing_radius: 3.0,
        path_length: 4.0,
        separation_distance: 2.0,
        separation_weight,
    }
}

impl LeaderFollowing {
    /// The leader at the origin facing +y, at rest; `count` followers at rest in a row 4 units
    /// behind it.
    fn new(count: usize, settings: LeaderFollow) -> Self {
        let followers = (0..count)
            .map(|i| Follower {
                vehicle: Vehicle {
                    position: Vec2::new((i as f32 - (count - 1) as f32 / 2.0) * 1.5, -4.0),
                    max_speed: 3.0,
                    max_force: 5.0,
                    ..Vehicle::default()
                },
                case: FollowCase::Follow,
                trail: Trail::default(),
            })
            .collect();
        Self {
            leader: Vehicle {
                max_speed: 1.5,
                max_force: 5.0,
                ..Vehicle::default()
            },
            wander_side: 0.0,
            rng: fastrand::Rng::with_seed(SEED),
            followers,
            settings,
            leader_trail: Trail::default(),
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let rng = &mut self.rng;
        let wander = steer::wander(&self.leader, &mut self.wander_side, dt, &mut || rng.f32());
        self.leader
            .apply_steering_force(self.leader.forward + wander * 3.0, dt);
        self.leader_trail.record(self.time, self.leader.position);
        for i in 0..self.followers.len() {
            let (force, case) = self.follower_force(i);
            let f = &mut self.followers[i];
            f.case = case;
            f.vehicle.apply_steering_force(force, dt);
            f.trail.record(self.time, f.vehicle.position);
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "followers" => *self = Self::new(FOLLOWERS.clamp(value) as usize, self.settings),
            "behind" => self.settings.behind = BEHIND.clamp(value),
            "separation" => self.settings.separation_weight = SEPARATION.clamp(value),
            _ => {}
        }
    }

    pub fn view(&self) -> FrameView {
        let l = &self.leader;
        let behind = behind_point(l, self.settings.behind);
        let mut lines = self.leader_path_lines();
        lines.extend(circle(behind, 0.3, color::TARGET));
        let first = &self.followers[0].vehicle;
        let (push, _) = self.follower_force(0);
        lines.push(line(first.position, first.position + push, color::PUSH));
        let stepping = self
            .followers
            .iter()
            .filter(|f| f.case == FollowCase::StepAside)
            .count();
        FrameView {
            time: self.time,
            focus: point(l.position),
            view_size: 30.0,
            vehicles: [vehicle_view(l, color::TARGET)]
                .into_iter()
                .chain(
                    self.followers
                        .iter()
                        .map(|f| vehicle_view(&f.vehicle, color::BODY)),
                )
                .collect(),
            lines,
            trails: [self.leader_trail.view(l.position)]
                .into_iter()
                .chain(
                    self.followers
                        .iter()
                        .map(|f| f.trail.view(f.vehicle.position)),
                )
                .collect(),
            labels: vec![
                label(l.position, "leader", color::LABEL),
                label(first.position, "follower 0", color::LABEL),
                label(
                    l.position + Vec2::new(0.0, -12.0),
                    format!("stepping aside: {stepping} of {}", self.followers.len()),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (color::TARGET, "leader, and the point behind it"),
                (color::RADIUS, "leader's path: followers step out of it"),
                (color::PUSH, "follower 0's push"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: Vec::new(),
            params: vec![
                slider(
                    "followers",
                    "followers",
                    FOLLOWERS,
                    self.followers.len() as f32,
                ),
                slider("behind", "distance behind", BEHIND, self.settings.behind),
                slider(
                    "separation",
                    "separation weight",
                    SEPARATION,
                    self.settings.separation_weight,
                ),
            ],
        }
    }

    /// Follower `i`'s force, separating from the leader and the other followers.
    fn follower_force(&self, i: usize) -> (Vec2, FollowCase) {
        let others = self
            .followers
            .iter()
            .enumerate()
            .filter_map(|(j, f)| (j != i).then_some(&f.vehicle))
            .chain([&self.leader]);
        follow_leader(
            &self.followers[i].vehicle,
            &self.leader,
            &self.settings,
            others,
        )
    }

    /// The rectangle `in_leader_path` tests, for a follower's radius.
    fn leader_path_lines(&self) -> Vec<LineView> {
        let l = &self.leader;
        let half_width = l.radius + 2.0 * self.followers[0].vehicle.radius;
        let (ahead, side) = (l.forward * self.settings.path_length, l.side() * half_width);
        let corners = [
            l.position + side,
            l.position + ahead + side,
            l.position + ahead - side,
            l.position - side,
        ];
        (0..4)
            .map(|k| line(corners[k], corners[(k + 1) % 4], color::RADIUS))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Over a minute: the followers' mean distance to the point behind, the fraction of
    /// follower-steps spent stepping aside, and the steps on which two followers overlap.
    fn measure(separation_weight: f32) -> (f32, f32, u32) {
        let mut demo = LeaderFollowing::new(8, settings(3.0, separation_weight));
        let (mut distance, mut aside, mut overlap_steps) = (0.0, 0, 0);
        let steps = 60 * 60;
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
            let behind = behind_point(&demo.leader, 3.0);
            let v: Vec<&Vehicle> = demo.followers.iter().map(|f| &f.vehicle).collect();
            distance += v
                .iter()
                .map(|f| (f.position - behind).length())
                .sum::<f32>();
            aside += demo
                .followers
                .iter()
                .filter(|f| f.case == FollowCase::StepAside)
                .count();
            let overlap = v.iter().enumerate().any(|(i, a)| {
                v[i + 1..]
                    .iter()
                    .any(|b| (a.position - b.position).length() < a.radius + b.radius)
            });
            overlap_steps += u32::from(overlap);
        }
        let follower_steps = (steps * 8) as f32;
        (
            distance / follower_steps,
            aside as f32 / follower_steps,
            overlap_steps,
        )
    }

    /// Separation (weight 2) keeps the 8 followers spread about 3.9 units around the point
    /// behind and cuts overlapping steps from 3524 of 3600 to 672; they step aside 3% of the
    /// time. Without it they pile onto the point behind (once two landed exactly on the same
    /// spot, where separation is 0/0).
    #[test]
    fn separation_spreads_the_followers() {
        let (distance, aside, overlaps) = measure(2.0);
        assert!((distance - 3.880).abs() < 0.01 && (aside - 0.0309).abs() < 0.001);
        assert_eq!(overlaps, 672);
        let (distance, aside, overlaps) = measure(0.0);
        assert!((distance - 3.206).abs() < 0.01 && (aside - 0.0014).abs() < 0.001);
        assert_eq!(overlaps, 3524);
    }

    #[test]
    fn followers_slider_restarts_with_that_many() {
        let mut demo = LeaderFollowing::default();
        demo.update(1.0 / 60.0);
        demo.param("followers", 3.0);
        assert_eq!(demo.followers.len(), 3);
        assert_eq!(demo.time, 0.0);
    }
}
