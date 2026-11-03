//! "Training Field": a Silkroad training spot in miniature, built to work on the lurer. A team
//! of six trains round one centre, each role keeping to its own radius: three dps (10), an HP
//! and an MP buffer (5) and our lurer (50); monsters spawn at random over a rectangular map.
//! Built so far (PLAN.md, Phase 8): the map, the disks, the spawns, and bodies that stand, walk
//! or run at human speeds, each wandering inside its own disk. Roles, combat and the lurer's
//! trips come in later steps.

use std::f32::consts::TAU;

use steer_core::containment::contain;
use steer_core::wall::Wall;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view};
use crate::controls::{button, slider, Range};
use crate::view::FrameView;

/// The map: a rectangle this wide and high, centred on the training centre.
pub const MAP_WIDTH: f32 = 160.0;
pub const MAP_HEIGHT: f32 = 120.0;
pub const CENTRE: Vec2 = Vec2::ZERO;
pub const DPS_RADIUS: f32 = 10.0;
pub const BUFF_RADIUS: f32 = 5.0;
pub const LURE_RADIUS: f32 = 50.0;
/// How far from its spawn an idle monster wanders.
pub const IDLE_RADIUS: f32 = 6.0;
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
/// Containment's look-ahead: the probe is `BODY_RADIUS + LOOK_AHEAD * speed` ahead.
const LOOK_AHEAD: f32 = 1.0;
/// A runner brakes while containment pushes (the Containment demo's finding): its turning
/// radius (speed squared over the force, about 3) is wider than the buffers' disk. A walker
/// turns within 0.3 and needs none.
const CONTAIN_BRAKE: f32 = 4.0;
/// Body colors: our lurer green, teammates blue, monsters red (muted, so the annotation
/// red and green stay the push and the want).
const LURER_BODY: u32 = 0x66aa66;
const TEAM_BODY: u32 = 0x6688cc;
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

/// How a body moves: at rest, or wandering at a human walk or run.
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

    const ALL: [Gait; 3] = [Gait::Standing, Gait::Walking, Gait::Running];

    fn id(self) -> &'static str {
        match self {
            Gait::Standing => "stand",
            Gait::Walking => "walk",
            Gait::Running => "run",
        }
    }

    fn name(self) -> &'static str {
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
    pub gait: Gait,
    pub body: Vehicle,
    wander_side: f32,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub gait: Gait,
    pub body: Vehicle,
    /// Where it appeared: it idles round here and (a later step) leashes back here.
    pub spawn: Vec2,
    wander_side: f32,
}

#[derive(Debug, Clone)]
pub struct TrainingField {
    /// The lurer, three dps, the HP buffer, the MP buffer.
    team: Vec<Player>,
    monsters: Vec<Monster>,
    /// How many monsters the map keeps alive (the slider).
    wanted: usize,
    /// The gait the buttons last gave the whole team.
    team_gait: Gait,
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

/// A disk as a containment region: a 40-point counter-clockwise polygon.
fn disk(centre: Vec2, radius: f32) -> Wall {
    const SIDES: usize = 40;
    let points: Vec<Vec2> = (0..SIDES)
        .map(|i| around(centre, radius, i as f32 * TAU / SIDES as f32))
        .collect();
    Wall::new(&points)
}

/// One step of `body` at `gait` inside `region`: standing stops it; walking and running wander
/// (the Containment demo's `forward + 3 wander`, the forward part a want of the gait's speed),
/// turned back by containment while the probe ahead is outside the region.
fn stroll(
    body: &mut Vehicle,
    gait: Gait,
    wander_side: &mut f32,
    region: &Wall,
    dt: f32,
    rng: &mut fastrand::Rng,
) {
    body.max_speed = gait.speed();
    let push = contain(body, region, LOOK_AHEAD);
    let force = if gait == Gait::Standing {
        Vec2::ZERO
    } else if push != Vec2::ZERO {
        if gait == Gait::Running {
            body.apply_braking_force(CONTAIN_BRAKE, dt);
        }
        push
    } else {
        let wander = steer::wander(body, wander_side, dt, &mut || rng.f32());
        steer::target_speed(body, gait.speed()) + wander * 3.0
    };
    body.apply_steering_force(force, dt);
}

impl Default for TrainingField {
    /// The team round the centre: the lurer on it, the dps 4 out at thirds of the circle,
    /// the buffers 2.5 out between them, all walking; `DEFAULT_MONSTERS` spawned.
    fn default() -> Self {
        let third = TAU / 3.0;
        let player = |role, at| Player {
            role,
            gait: Gait::Walking,
            body: standing(at),
            wander_side: 0.0,
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
            team_gait: Gait::Walking,
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

    /// Spawns the missing monsters, then moves every body: a player inside its role's disk, a
    /// monster inside `IDLE_RADIUS` of its spawn, each with its whole body inside.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.spawn_missing();
        let rng = &mut self.rng;
        for p in &mut self.team {
            let region = disk(CENTRE, p.role.radius() - BODY_RADIUS);
            stroll(&mut p.body, p.gait, &mut p.wander_side, &region, dt, rng);
        }
        for m in &mut self.monsters {
            let region = disk(m.spawn, IDLE_RADIUS - BODY_RADIUS);
            stroll(&mut m.body, m.gait, &mut m.wander_side, &region, dt, rng);
        }
    }

    /// The gait buttons: the whole team stands, walks or runs.
    pub fn action(&mut self, id: &str) {
        if let Some(gait) = Gait::ALL.into_iter().find(|g| g.id() == id) {
            self.team_gait = gait;
            for p in &mut self.team {
                p.gait = gait;
            }
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "monsters" {
            self.wanted = MONSTERS.clamp(value) as usize;
            self.monsters.truncate(self.wanted);
            self.spawn_missing();
        }
    }

    /// Spawns monsters until `wanted` are on the map, walking round their spawn.
    fn spawn_missing(&mut self) {
        while self.monsters.len() < self.wanted {
            let at = self.spawn_point();
            self.monsters.push(Monster {
                gait: Gait::Walking,
                body: standing(at),
                spawn: at,
                wander_side: 0.0,
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
        let mut labels = Vec::new();
        for (r, text) in [
            (BUFF_RADIUS, "buffers 5"),
            (DPS_RADIUS, "dps 10"),
            (LURE_RADIUS, "lurer 50"),
        ] {
            lines.extend(circle(CENTRE, r, color::RADIUS));
            labels.push(label(CENTRE + Vec2::new(0.0, -r - 1.0), text, color::LABEL));
        }
        labels.push(label(
            corner(0.0, h + 3.0),
            format!(
                "{} monsters walking; the team {} ({} m/s)",
                self.monsters.len(),
                self.team_gait.name(),
                self.team_gait.speed()
            ),
            color::LABEL,
        ));
        let body = |p: &Player| match p.role {
            Role::Lurer => LURER_BODY,
            _ => TEAM_BODY,
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
                (TEAM_BODY, "teammates: 3 dps, HP buffer, MP buffer"),
                (MONSTER_BODY, "monster"),
                (
                    color::RADIUS,
                    "training disks: each role's radius round the centre",
                ),
                (color::EDGE, "map edge: monsters spawn inside"),
            ]),
            actions: Gait::ALL
                .iter()
                .map(|g| button(g.id(), g.name(), *g == self.team_gait))
                .collect(),
            params: vec![slider("monsters", "Monsters", MONSTERS, self.wanted as f32)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(field: &mut TrainingField, seconds: f32) {
        for _ in 0..(seconds * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
    }

    /// Over `seconds` after a 10 s warm-up: the furthest any player's centre gets past the ring
    /// its body must stay inside (`role.radius() - BODY_RADIUS`), the furthest any monster gets
    /// from its spawn, and each player's mean and lowest speed.
    struct Measured {
        player_over: f32,
        monster_out: f32,
        mean_speed: [f32; 6],
        min_speed: [f32; 6],
    }

    fn measure(field: &mut TrainingField, seconds: f32) -> Measured {
        const WARM_UP: usize = 600;
        let mut m = Measured {
            player_over: f32::MIN,
            monster_out: 0.0,
            mean_speed: [0.0; 6],
            min_speed: [f32::MAX; 6],
        };
        let steps = (seconds * 60.0) as usize;
        for step in 0..steps {
            field.update(1.0 / 60.0);
            for (i, p) in field.team().iter().enumerate() {
                let over = (p.body.position - CENTRE).length() - (p.role.radius() - BODY_RADIUS);
                m.player_over = m.player_over.max(over);
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

    #[test]
    fn walking_bodies_hold_walking_speed_inside_their_disks() {
        let mut field = TrainingField::default();
        let m = measure(&mut field, 120.0);
        assert!(m.player_over <= 0.0, "{}", m.player_over);
        assert!(
            m.monster_out <= IDLE_RADIUS - BODY_RADIUS,
            "{}",
            m.monster_out
        );
        for speed in m.mean_speed {
            assert!((speed - WALK_SPEED).abs() < 0.01, "{speed}");
        }
        for monster in field.monsters() {
            assert!((monster.body.speed - WALK_SPEED).abs() < 0.01);
        }
        // They did wander: no teammate is still at its start.
        let moved = (field.team().iter())
            .filter(|p| (p.body.position - CENTRE).length() > 1.0 || p.role == Role::Lurer);
        assert_eq!(moved.count(), 6);
    }

    /// Running inside a small disk is a run braked into every turn: the lurer, with room,
    /// averages 4.6; the dps 3.4 in their 10; the buffers 2.1 in their 5; nobody below 1.47.
    #[test]
    fn the_buttons_make_the_team_run_or_stand() {
        let mut field = TrainingField::default();
        field.action("run");
        let m = measure(&mut field, 60.0);
        assert!(m.player_over <= 0.0, "{}", m.player_over);
        assert!(m.mean_speed[0] > 4.5, "{:?}", m.mean_speed);
        assert!(
            m.mean_speed[1..4].iter().all(|&s| s > 3.0),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.mean_speed[4..].iter().all(|&s| s > 2.0),
            "{:?}",
            m.mean_speed
        );
        assert!(m.min_speed.iter().all(|&s| s > 1.4), "{:?}", m.min_speed);
        assert!(field
            .team()
            .iter()
            .all(|p| p.body.speed <= RUN_SPEED + 1e-3));
        let active: Vec<_> = (field.view().actions.iter())
            .filter(|a| a.active)
            .map(|a| a.id.clone())
            .collect();
        assert_eq!(active, ["run"]);
        field.action("stand");
        let before: Vec<Vec2> = field.team().iter().map(|p| p.body.position).collect();
        run(&mut field, 1.0);
        for (p, at) in field.team().iter().zip(before) {
            assert_eq!(p.body.speed, 0.0);
            assert!((p.body.position - at).length() < RUN_SPEED / 60.0);
        }
    }
}
