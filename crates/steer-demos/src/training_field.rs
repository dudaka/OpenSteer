//! "Training Field": a Silkroad training spot in miniature, built to work on the lurer. A team
//! of six trains round one centre, each role keeping to its own radius: three dps (10), an HP
//! and an MP buffer (5) and our lurer (50); monsters spawn at random over a rectangular map.
//! Built so far (PLAN.md, Phase 8): the map, the disks, the spawns, and bodies at human speeds,
//! each roaming its own disk from one random spot to the next: the players always run, a
//! monster stands, walks or runs for a few seconds at a time. Every entity looks ahead through a fan of rays (`sight`), each
//! cut at the first body it meets. Roles, combat and the lurer's trips come in later steps.

use std::f32::consts::TAU;

use steer_core::containment::contain;
use steer_core::wall::Wall;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view};
use crate::controls::{slider, Range};
use crate::view::FrameView;

/// The map: a rectangle this wide and high, centred on the training centre.
pub const MAP_WIDTH: f32 = 160.0;
pub const MAP_HEIGHT: f32 = 120.0;
pub const CENTRE: Vec2 = Vec2::ZERO;
pub const DPS_RADIUS: f32 = 10.0;
pub const BUFF_RADIUS: f32 = 5.0;
pub const LURE_RADIUS: f32 = 50.0;
/// How far from its spawn an idle monster wanders, and how long it keeps a gait.
pub const IDLE_RADIUS: f32 = 6.0;
const GAIT_MIN_S: f32 = 2.0;
const GAIT_MAX_S: f32 = 6.0;
const MONSTERS: Range = Range::new(5.0, 60.0, 1.0);
const DEFAULT_MONSTERS: usize = 30;
const SEED: u64 = 1;
/// A body seen from above, 2 units across: a person at the map's scale (OpenSteer's default
/// 0.5 reads as a dot on a 160-unit map).
pub const BODY_RADIUS: f32 = 1.0;
/// Human speeds, 1 unit = 1 m: a walk and a run.
pub const WALK_SPEED: f32 = 1.5;
pub const RUN_SPEED: f32 = 5.0;
/// Pedestrians' max force.
const MAX_FORCE: f32 = 8.0;
/// Sight: a fan of `FAN_RAYS` rays spread over `FAN_ANGLE` ahead, each `SIGHT_RANGE` long.
pub const SIGHT_RANGE: f32 = 12.0;
pub const FAN_RAYS: usize = 7;
pub const FAN_ANGLE: f32 = TAU / 4.0;
/// Containment's look-ahead: the probe is `BODY_RADIUS + LOOK_AHEAD * speed` ahead.
const LOOK_AHEAD: f32 = 1.0;
/// A roaming body counts its spot as reached this close to it and draws the next.
const SPOT_REACHED: f32 = 1.5;
/// A runner brakes while containment pushes (the Containment demo's finding): its turning
/// radius (speed squared over the force, about 3) is wider than the buffers' disk. A walker
/// turns within 0.3 and needs none.
const CONTAIN_BRAKE: f32 = 4.0;
/// Body colors: our lurer green, the dps blue, the HP buffer rose, the MP buffer teal,
/// monsters red (muted, so the annotation red and green stay the push and the want).
const LURER_BODY: u32 = 0x66aa66;
const DPS_BODY: u32 = 0x6688cc;
const HP_BUFFER_BODY: u32 = 0xcc6699;
const MP_BUFFER_BODY: u32 = 0x55aaaa;
const MONSTER_BODY: u32 = 0x996666;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Lurer,
    Dps,
    HpBuffer,
    MpBuffer,
}

impl Role {
    /// The disk this role keeps to, round the shared centre.
    pub fn radius(self) -> f32 {
        match self {
            Role::Lurer => LURE_RADIUS,
            Role::Dps => DPS_RADIUS,
            Role::HpBuffer | Role::MpBuffer => BUFF_RADIUS,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Role::Lurer => "lurer",
            Role::Dps => "dps",
            Role::HpBuffer => "HP buffer",
            Role::MpBuffer => "MP buffer",
        }
    }
}

/// How a body moves: at rest, or wandering at a human walk or run. Players always run; a
/// monster draws a gait every few seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    Standing,
    Walking,
    Running,
}

impl Gait {
    pub fn speed(self) -> f32 {
        match self {
            Gait::Standing => 0.0,
            Gait::Walking => WALK_SPEED,
            Gait::Running => RUN_SPEED,
        }
    }

    pub const ALL: [Gait; 3] = [Gait::Standing, Gait::Walking, Gait::Running];

    pub fn name(self) -> &'static str {
        match self {
            Gait::Standing => "standing",
            Gait::Walking => "walking",
            Gait::Running => "running",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub role: Role,
    pub body: Vehicle,
    /// The spot it is running to inside its disk.
    spot: Vec2,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub gait: Gait,
    /// When the next gait is drawn (simulation time).
    until: f32,
    pub body: Vehicle,
    /// Where it appeared: it idles round here and (a later step) leashes back here.
    pub spawn: Vec2,
    /// The spot it is going to inside its idle disk.
    spot: Vec2,
}

#[derive(Debug, Clone)]
pub struct TrainingField {
    /// The lurer, three dps, the HP buffer, the MP buffer.
    team: Vec<Player>,
    monsters: Vec<Monster>,
    /// How many monsters the map keeps alive (the slider).
    wanted: usize,
    rng: fastrand::Rng,
    time: f32,
}

/// A body at rest at `position`, facing up the screen.
fn standing(position: Vec2) -> Vehicle {
    Vehicle {
        position,
        forward: Vec2::new(0.0, -1.0),
        radius: BODY_RADIUS,
        max_force: MAX_FORCE,
        ..Vehicle::default()
    }
}

/// A point `r` from `centre` at `angle` (radians).
fn around(centre: Vec2, r: f32, angle: f32) -> Vec2 {
    centre + Vec2::new(angle.cos(), angle.sin()) * r
}

/// One ray of a fan: from the eye to the first body it meets, or to the end of its range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub from: Vec2,
    pub to: Vec2,
    /// The entity hit, as an index over the team then the monsters.
    pub hit: Option<usize>,
}

/// The fan's unit directions: `FAN_RAYS` over `FAN_ANGLE`, centred on `forward`.
pub fn fan(forward: Vec2) -> [Vec2; FAN_RAYS] {
    std::array::from_fn(|i| {
        let angle = FAN_ANGLE * (i as f32 / (FAN_RAYS - 1) as f32 - 0.5);
        let (sin, cos) = angle.sin_cos();
        Vec2::new(
            forward.x * cos - forward.y * sin,
            forward.x * sin + forward.y * cos,
        )
    })
}

/// How far along a ray from `from` in unit direction `dir` the circle (`centre`, `radius`)
/// is first entered, if within `range`: the near root of the ray-circle intersection.
pub fn ray_enters_circle(
    from: Vec2,
    dir: Vec2,
    range: f32,
    centre: Vec2,
    radius: f32,
) -> Option<f32> {
    let to_centre = centre - from;
    let along = to_centre.dot(dir);
    let off_squared = to_centre.length_squared() - along * along;
    if off_squared > radius * radius {
        return None;
    }
    let t = along - (radius * radius - off_squared).sqrt();
    (0.0..=range).contains(&t).then_some(t)
}

/// A disk as a containment region: a 40-point counter-clockwise polygon.
fn disk(centre: Vec2, radius: f32) -> Wall {
    const SIDES: usize = 40;
    let points: Vec<Vec2> = (0..SIDES)
        .map(|i| around(centre, radius, i as f32 * TAU / SIDES as f32))
        .collect();
    Wall::new(&points)
}

/// A random point of the disk (`centre`, `radius`), uniform over its area.
fn random_spot(centre: Vec2, radius: f32, rng: &mut fastrand::Rng) -> Vec2 {
    around(centre, radius * rng.f32().sqrt(), rng.f32() * TAU)
}

/// One step of `body` at `gait` roaming the disk (`centre`, `radius`) with its whole body
/// inside: standing stops it; otherwise it seeks `spot`, drawing a new random spot of the disk
/// each time it comes within `SPOT_REACHED` (the game bots' random walk, which covers the whole
/// disk; a wanderer held by containment only circles along the edge). Containment stays as the
/// guard while the probe ahead is outside, a runner braking into it.
fn roam(
    body: &mut Vehicle,
    gait: Gait,
    spot: &mut Vec2,
    centre: Vec2,
    radius: f32,
    dt: f32,
    rng: &mut fastrand::Rng,
) {
    let room = radius - BODY_RADIUS;
    body.max_speed = gait.speed();
    if (*spot - body.position).length() < SPOT_REACHED {
        *spot = random_spot(centre, room, rng);
    }
    let push = contain(body, &disk(centre, room), LOOK_AHEAD);
    let force = if gait == Gait::Standing {
        Vec2::ZERO
    } else if push != Vec2::ZERO {
        if gait == Gait::Running {
            body.apply_braking_force(CONTAIN_BRAKE, dt);
        }
        push
    } else {
        steer::seek(body, *spot)
    };
    body.apply_steering_force(force, dt);
}

impl Default for TrainingField {
    /// The team round the centre: the lurer on it, the dps 4 out at thirds of the circle,
    /// the buffers 2.5 out between them; `DEFAULT_MONSTERS` spawned.
    fn default() -> Self {
        let third = TAU / 3.0;
        let player = |role, at| Player {
            role,
            body: standing(at),
            spot: at,
        };
        let team = vec![
            player(Role::Lurer, CENTRE),
            player(Role::Dps, around(CENTRE, 4.0, 0.0)),
            player(Role::Dps, around(CENTRE, 4.0, third)),
            player(Role::Dps, around(CENTRE, 4.0, 2.0 * third)),
            player(Role::HpBuffer, around(CENTRE, 2.5, third / 2.0)),
            player(Role::MpBuffer, around(CENTRE, 2.5, -third / 2.0)),
        ];
        let mut field = Self {
            team,
            monsters: Vec::new(),
            wanted: DEFAULT_MONSTERS,
            rng: fastrand::Rng::with_seed(SEED),
            time: 0.0,
        };
        field.spawn_missing();
        field
    }
}

impl TrainingField {
    pub fn team(&self) -> &[Player] {
        &self.team
    }

    pub fn monsters(&self) -> &[Monster] {
        &self.monsters
    }

    /// Every body, the team first, then the monsters: the index space of `Ray::hit`.
    fn bodies(&self) -> impl Iterator<Item = &Vehicle> {
        (self.team.iter().map(|p| &p.body)).chain(self.monsters.iter().map(|m| &m.body))
    }

    /// What entity `eye` (an index over `bodies`) sees: its fan of rays, each cut at the
    /// nearest other body it enters.
    pub fn sight(&self, eye: usize) -> [Ray; FAN_RAYS] {
        let me = self.bodies().nth(eye).expect("an entity");
        fan(me.forward).map(|dir| {
            let nearest = (self.bodies().enumerate())
                .filter(|&(i, _)| i != eye)
                .filter_map(|(i, b)| {
                    ray_enters_circle(me.position, dir, SIGHT_RANGE, b.position, b.radius)
                        .map(|t| (i, t))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let (hit, reach) = nearest.map_or((None, SIGHT_RANGE), |(i, t)| (Some(i), t));
            Ray {
                from: me.position,
                to: me.position + dir * reach,
                hit,
            }
        })
    }

    /// Spawns the missing monsters, then moves every body: a player runs inside its role's
    /// disk, a monster keeps inside `IDLE_RADIUS` of its spawn at the gait it drew for the
    /// next `GAIT_MIN_S`-`GAIT_MAX_S` seconds, each with its whole body inside.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.spawn_missing();
        let rng = &mut self.rng;
        for p in &mut self.team {
            let r = p.role.radius();
            roam(&mut p.body, Gait::Running, &mut p.spot, CENTRE, r, dt, rng);
        }
        for m in &mut self.monsters {
            if self.time >= m.until {
                m.gait = Gait::ALL[rng.usize(..Gait::ALL.len())];
                m.until = self.time + GAIT_MIN_S + (GAIT_MAX_S - GAIT_MIN_S) * rng.f32();
            }
            roam(
                &mut m.body,
                m.gait,
                &mut m.spot,
                m.spawn,
                IDLE_RADIUS,
                dt,
                rng,
            );
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "monsters" {
            self.wanted = MONSTERS.clamp(value) as usize;
            self.monsters.truncate(self.wanted);
            self.spawn_missing();
        }
    }

    /// Spawns monsters until `wanted` are on the map, standing until their first gait draw.
    fn spawn_missing(&mut self) {
        while self.monsters.len() < self.wanted {
            let at = self.spawn_point();
            self.monsters.push(Monster {
                gait: Gait::Standing,
                until: self.time,
                body: standing(at),
                spawn: at,
                spot: at,
            });
        }
    }

    /// A random point of the map outside the dps disk (nothing appears on top of the team).
    fn spawn_point(&mut self) -> Vec2 {
        loop {
            let at = CENTRE
                + Vec2::new(
                    (self.rng.f32() - 0.5) * MAP_WIDTH,
                    (self.rng.f32() - 0.5) * MAP_HEIGHT,
                );
            if (at - CENTRE).length() > DPS_RADIUS {
                return at;
            }
        }
    }

    pub fn view(&self) -> FrameView {
        let (w, h) = (MAP_WIDTH / 2.0, MAP_HEIGHT / 2.0);
        let corner = |x: f32, y: f32| CENTRE + Vec2::new(x, y);
        let corners = [
            corner(-w, -h),
            corner(w, -h),
            corner(w, h),
            corner(-w, h),
            corner(-w, -h),
        ];
        let mut lines: Vec<_> = corners
            .windows(2)
            .map(|c| line(c[0], c[1], color::EDGE))
            .collect();
        let count = self.team.len() + self.monsters.len();
        lines.extend(
            (0..count)
                .flat_map(|eye| self.sight(eye))
                .map(|ray| line(ray.from, ray.to, color::SIGHT)),
        );
        let mut labels = Vec::new();
        for (r, text) in [
            (BUFF_RADIUS, "buffers 5"),
            (DPS_RADIUS, "dps 10"),
            (LURE_RADIUS, "lurer 50"),
        ] {
            lines.extend(circle(CENTRE, r, color::RADIUS));
            labels.push(label(CENTRE + Vec2::new(0.0, -r - 1.0), text, color::LABEL));
        }
        let gaits: Vec<String> = (Gait::ALL.iter())
            .map(|g| {
                let n = self.monsters.iter().filter(|m| m.gait == *g).count();
                format!("{n} {}", g.name())
            })
            .collect();
        labels.push(label(
            corner(0.0, h + 3.0),
            format!("{} monsters: {}", self.monsters.len(), gaits.join(", ")),
            color::LABEL,
        ));
        let body = |p: &Player| match p.role {
            Role::Lurer => LURER_BODY,
            Role::Dps => DPS_BODY,
            Role::HpBuffer => HP_BUFFER_BODY,
            Role::MpBuffer => MP_BUFFER_BODY,
        };
        FrameView {
            time: self.time,
            focus: [CENTRE.x, CENTRE.y],
            view_size: MAP_HEIGHT + 12.0,
            vehicles: (self.team.iter().map(|p| vehicle_view(&p.body, body(p))))
                .chain(
                    self.monsters
                        .iter()
                        .map(|m| vehicle_view(&m.body, MONSTER_BODY)),
                )
                .collect(),
            lines,
            trails: Vec::new(),
            labels,
            legend: legend(&[
                (LURER_BODY, "our lurer"),
                (DPS_BODY, "dps (3)"),
                (HP_BUFFER_BODY, "HP buffer"),
                (MP_BUFFER_BODY, "MP buffer"),
                (MONSTER_BODY, "monster"),
                (
                    color::RADIUS,
                    "training disks: each role's radius round the centre",
                ),
                (color::EDGE, "map edge: monsters spawn inside"),
                (
                    color::SIGHT,
                    "sight: 7 rays over 90 degrees ahead, 12 long, cut at the first body",
                ),
            ]),
            actions: Vec::new(),
            params: vec![slider("monsters", "Monsters", MONSTERS, self.wanted as f32)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Over `seconds` after a 10 s warm-up: the furthest any player's centre gets past the ring
    /// its body must stay inside (`role.radius() - BODY_RADIUS`), the furthest any monster gets
    /// from its spawn, and each player's mean and lowest speed.
    struct Measured {
        player_over: f32,
        monster_out: f32,
        mean_speed: [f32; 6],
        min_speed: [f32; 6],
        /// The share of steps each player spent inside the buffers' disk.
        in_buff_disk: [f32; 6],
    }

    fn measure(field: &mut TrainingField, seconds: f32) -> Measured {
        const WARM_UP: usize = 600;
        let mut m = Measured {
            player_over: f32::MIN,
            monster_out: 0.0,
            mean_speed: [0.0; 6],
            min_speed: [f32::MAX; 6],
            in_buff_disk: [0.0; 6],
        };
        let steps = (seconds * 60.0) as usize;
        for step in 0..steps {
            field.update(1.0 / 60.0);
            for (i, p) in field.team().iter().enumerate() {
                let over = (p.body.position - CENTRE).length() - (p.role.radius() - BODY_RADIUS);
                m.player_over = m.player_over.max(over);
                if (p.body.position - CENTRE).length() <= BUFF_RADIUS {
                    m.in_buff_disk[i] += 1.0 / steps as f32;
                }
                if step >= WARM_UP {
                    m.mean_speed[i] += p.body.speed / (steps - WARM_UP) as f32;
                    m.min_speed[i] = m.min_speed[i].min(p.body.speed);
                }
            }
            for monster in field.monsters() {
                m.monster_out = m
                    .monster_out
                    .max((monster.body.position - monster.spawn).length());
            }
        }
        m
    }

    #[test]
    fn a_ray_enters_a_circle_at_its_near_side() {
        let (from, up) = (Vec2::ZERO, Vec2::new(0.0, 1.0));
        // A body of radius 1 centred 5 ahead is entered at 4.
        assert_eq!(
            ray_enters_circle(from, up, 12.0, Vec2::new(0.0, 5.0), 1.0),
            Some(4.0)
        );
        // Off to the side by 0.6 of radius 1: entered at 5 - 0.8.
        let t = ray_enters_circle(from, up, 12.0, Vec2::new(0.6, 5.0), 1.0).unwrap();
        assert!((t - 4.2).abs() < 1e-6, "{t}");
        // Off by more than the radius, behind, or beyond the range: nothing.
        assert_eq!(
            ray_enters_circle(from, up, 12.0, Vec2::new(1.5, 5.0), 1.0),
            None
        );
        assert_eq!(
            ray_enters_circle(from, up, 12.0, Vec2::new(0.0, -5.0), 1.0),
            None
        );
        assert_eq!(
            ray_enters_circle(from, up, 12.0, Vec2::new(0.0, 14.0), 1.0),
            None
        );
    }

    #[test]
    fn the_fan_spans_the_angle_symmetrically() {
        let rays = fan(Vec2::new(0.0, 1.0));
        assert_eq!(rays.len(), FAN_RAYS);
        for (i, r) in rays.iter().enumerate() {
            assert!((r.length() - 1.0).abs() < 1e-6);
            let mirrored = rays[FAN_RAYS - 1 - i];
            assert!((r.x + mirrored.x).abs() < 1e-6 && (r.y - mirrored.y).abs() < 1e-6);
        }
        // The first ray is turned by -FAN_ANGLE / 2: for forward (0, 1) that is toward +x.
        let half = (FAN_ANGLE / 2.0).sin();
        assert!((rays[0].x - half).abs() < 1e-6, "{:?}", rays[0]);
        assert!((rays[FAN_RAYS / 2].y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sight_is_cut_at_the_nearest_body_and_never_at_itself() {
        let mut field = TrainingField::default();
        // The lurer at the centre faces up the screen (-y); put a monster 6 ahead and one 9.
        field.param("monsters", 5.0);
        field.monsters[0].body.position = Vec2::new(0.0, -6.0);
        field.monsters[1].body.position = Vec2::new(0.0, -9.0);
        let rays = field.sight(0);
        let middle = rays[FAN_RAYS / 2];
        assert_eq!(middle.hit, Some(6));
        assert_eq!(middle.to, Vec2::new(0.0, -5.0));
        for _ in 0..600 {
            field.update(1.0 / 60.0);
        }
        let count = field.team().len() + field.monsters().len();
        for eye in 0..count {
            for ray in field.sight(eye) {
                assert_ne!(ray.hit, Some(eye));
                assert!((ray.to - ray.from).length() <= SIGHT_RANGE + 1e-4);
            }
        }
        let drawn = field
            .view()
            .lines
            .iter()
            .filter(|l| l.color == color::SIGHT)
            .count();
        assert_eq!(drawn, count * FAN_RAYS);
    }

    #[test]
    fn the_team_is_six_roles_standing_inside_their_disks() {
        let field = TrainingField::default();
        let roles: Vec<Role> = field.team().iter().map(|p| p.role).collect();
        assert_eq!(
            roles,
            [
                Role::Lurer,
                Role::Dps,
                Role::Dps,
                Role::Dps,
                Role::HpBuffer,
                Role::MpBuffer
            ]
        );
        for p in field.team() {
            assert!((p.body.position - CENTRE).length() <= p.role.radius());
            assert_eq!(p.body.speed, 0.0);
            assert_eq!(p.body.radius, BODY_RADIUS);
        }
        // No two bodies overlap.
        for (i, a) in field.team().iter().enumerate() {
            for b in &field.team()[i + 1..] {
                assert!((a.body.position - b.body.position).length() > 2.0 * BODY_RADIUS);
            }
        }
    }

    #[test]
    fn monsters_spawn_over_the_map_but_not_in_the_dps_disk() {
        let field = TrainingField::default();
        assert_eq!(field.monsters().len(), DEFAULT_MONSTERS);
        for m in field.monsters() {
            let off = m.body.position - CENTRE;
            assert!(off.x.abs() <= MAP_WIDTH / 2.0 && off.y.abs() <= MAP_HEIGHT / 2.0);
            assert!(off.length() > DPS_RADIUS);
            assert_eq!(m.spawn, m.body.position);
        }
        // They spread: some in the lurer's disk, some beyond it.
        let in_reach = field
            .monsters()
            .iter()
            .filter(|m| (m.body.position - CENTRE).length() <= LURE_RADIUS)
            .count();
        assert!(in_reach > 5 && in_reach < DEFAULT_MONSTERS);
    }

    #[test]
    fn the_slider_keeps_the_count() {
        let mut field = TrainingField::default();
        field.param("monsters", 45.0);
        assert_eq!(field.monsters().len(), 45);
        field.param("monsters", 8.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.monsters().len(), 8);
        field.param("monsters", 1000.0);
        assert_eq!(field.monsters().len(), 60);
    }

    /// Roaming spot to spot is a run slowed at every turn-back and braked into the edge: the
    /// lurer, with room, averages 4.3; the dps 2.7 in their 10; the buffers 1.5 in their 5;
    /// nobody below 0.76. Spots cover the disk: the dps spend 42-49% of the time inside the
    /// buffers' disk (the user 2026-11-02: with wander they never came in), the lurer 5%.
    #[test]
    fn players_run_inside_their_disks() {
        let mut field = TrainingField::default();
        let m = measure(&mut field, 60.0);
        assert!(m.player_over <= 0.0, "{}", m.player_over);
        assert!(m.mean_speed[0] > 4.0, "{:?}", m.mean_speed);
        assert!(
            m.mean_speed[1..4].iter().all(|&s| s > 2.5),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.mean_speed[4..].iter().all(|&s| s > 1.4),
            "{:?}",
            m.mean_speed
        );
        assert!(m.min_speed.iter().all(|&s| s > 0.7), "{:?}", m.min_speed);
        assert!(
            m.in_buff_disk[1..4].iter().all(|&s| s > 0.3),
            "{:?}",
            m.in_buff_disk
        );
        assert!(
            m.in_buff_disk[4..].iter().all(|&s| s > 0.99),
            "{:?}",
            m.in_buff_disk
        );
        assert!(m.in_buff_disk[0] < 0.1, "{:?}", m.in_buff_disk);
        assert!(field
            .team()
            .iter()
            .all(|p| p.body.speed <= RUN_SPEED + 1e-3));
    }

    /// A monster keeps each gait 2-6 s: over two minutes every monster has stood, walked and
    /// run, each gait about a third of the time, never faster than its gait, never further
    /// than its idle radius from the spawn.
    #[test]
    fn monsters_change_gait_and_keep_near_their_spawn() {
        let mut field = TrainingField::default();
        let mut steps_in = [0usize; 3];
        let mut visited = [[false; 3]; DEFAULT_MONSTERS];
        let mut out = 0.0f32;
        for _ in 0..120 * 60 {
            field.update(1.0 / 60.0);
            for (i, m) in field.monsters().iter().enumerate() {
                let g = Gait::ALL.iter().position(|&g| g == m.gait).unwrap();
                steps_in[g] += 1;
                visited[i][g] = true;
                assert!(
                    m.body.speed <= m.gait.speed() + 1e-3,
                    "{:?} at {}",
                    m.gait,
                    m.body.speed
                );
                out = out.max((m.body.position - m.spawn).length());
            }
        }
        let total = steps_in.iter().sum::<usize>() as f32;
        let shares = steps_in.map(|n| n as f32 / total);
        assert!(shares.iter().all(|&s| s > 0.25 && s < 0.42), "{shares:?}");
        assert!(visited.iter().all(|v| v.iter().all(|&b| b)));
        assert!(out <= IDLE_RADIUS - BODY_RADIUS, "{out}");
    }
}
