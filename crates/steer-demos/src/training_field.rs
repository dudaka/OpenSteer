//! "Training Field": a Silkroad training spot in miniature, built to work on the lurer. A team
//! of six trains round one centre, each role keeping to its own radius: three dps (10), an HP
//! and an MP buffer (5) and our lurer (50); monsters spawn at random over a rectangular map.
//! Built so far (PLAN.md, Phase 8): the map, the disks, the spawns, and bodies at human speeds,
//! each roaming its own disk from one random spot to the next: the players always run, a
//! monster stands, walks or runs for a few seconds at a time. Every entity carries two disks
//! on its own position: a big one, its ranged reach, and a small one, its melee reach. Players
//! are ranged; a monster is melee, and chases the nearest player inside its big disk until that
//! player is inside its small disk. Combat and the lurer's trips come in later steps.

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
/// Every entity's two reaches, centre to centre: ranged (the big disk; a player's attack, a
/// monster's alarm) and melee (the small disk; a monster's attack).
pub const RANGED_REACH: f32 = 8.0;
pub const MELEE_REACH: f32 = 2.0;
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
    /// The player (an index into the team) inside its big disk that it is closing on.
    pub target: Option<usize>,
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
    // Outside the disk (a monster back from a chase) the spot leads home; no guard.
    let push = if (body.position - centre).length() <= room {
        contain(body, &disk(centre, room), LOOK_AHEAD)
    } else {
        Vec2::ZERO
    };
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

/// One step of a monster closing on `target`: a run toward it while it is outside melee reach,
/// a stop (facing it) once inside.
fn hunt(body: &mut Vehicle, target: Vec2, dt: f32) -> Gait {
    let gait = if (target - body.position).length() > MELEE_REACH {
        Gait::Running
    } else {
        Gait::Standing
    };
    body.max_speed = gait.speed();
    let force = match gait {
        Gait::Running => steer::seek(body, target),
        _ => Vec2::ZERO,
    };
    body.apply_steering_force(force, dt);
    gait
}

/// The nearest of `players` within `RANGED_REACH` of `at`, and its distance.
fn player_in_reach(at: Vec2, players: &[Vec2]) -> Option<(usize, f32)> {
    (players.iter().enumerate())
        .map(|(i, &p)| (i, (p - at).length()))
        .filter(|&(_, d)| d <= RANGED_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
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

    /// Spawns the missing monsters, then moves every body: a player runs inside its role's
    /// disk; a monster with a player inside its big disk closes on the nearest one (`hunt`),
    /// otherwise keeps inside `IDLE_RADIUS` of its spawn at the gait it drew for the next
    /// `GAIT_MIN_S`-`GAIT_MAX_S` seconds, each with its whole body inside.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.spawn_missing();
        let rng = &mut self.rng;
        for p in &mut self.team {
            let r = p.role.radius();
            roam(&mut p.body, Gait::Running, &mut p.spot, CENTRE, r, dt, rng);
        }
        let players: Vec<Vec2> = self.team.iter().map(|p| p.body.position).collect();
        for m in &mut self.monsters {
            m.target = player_in_reach(m.body.position, &players).map(|(i, _)| i);
            if let Some(i) = m.target {
                m.gait = hunt(&mut m.body, players[i], dt);
                // A fresh gait draw as soon as the chase ends.
                m.until = self.time;
                continue;
            }
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
                target: None,
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
        let bodies =
            (self.team.iter().map(|p| &p.body)).chain(self.monsters.iter().map(|m| &m.body));
        for b in bodies {
            lines.extend(circle(b.position, RANGED_REACH, color::RANGED));
            lines.extend(circle(b.position, MELEE_REACH, color::MELEE));
        }
        for m in &self.monsters {
            if let Some(i) = m.target {
                lines.push(line(
                    m.body.position,
                    self.team[i].body.position,
                    color::TARGET,
                ));
            }
        }
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
        let chasing = self.monsters.iter().filter(|m| m.target.is_some()).count();
        labels.push(label(
            corner(0.0, h + 3.0),
            format!(
                "{} monsters: {}; {chasing} chasing",
                self.monsters.len(),
                gaits.join(", ")
            ),
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
                    color::RANGED,
                    "ranged reach 8 round each entity: a player's attack, a monster's alarm",
                ),
                (color::MELEE, "melee reach 2: a monster's attack"),
                (color::TARGET, "chase: the player a monster closes on"),
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
    fn a_monster_closes_on_its_target_and_stops_at_melee_reach() {
        // 7 away, within the big disk: a run that ends at the small disk's edge.
        let mut body = standing(Vec2::ZERO);
        let target = Vec2::new(0.0, -7.0);
        let mut gaits = Vec::new();
        for _ in 0..180 {
            gaits.push(hunt(&mut body, target, 1.0 / 60.0));
        }
        let gap = (target - body.position).length();
        assert!(gap <= MELEE_REACH && gap > MELEE_REACH - 0.1, "{gap}");
        assert_eq!(body.speed, 0.0);
        assert_eq!(gaits[0], Gait::Running);
        assert_eq!(gaits[179], Gait::Standing);
        // Only players inside the big disk count, the nearest first.
        let players = [
            Vec2::new(0.0, -9.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(-3.0, 0.0),
        ];
        assert_eq!(player_in_reach(Vec2::ZERO, &players), Some((2, 3.0)));
        assert_eq!(player_in_reach(Vec2::new(0.0, 20.0), &players), None);
    }

    #[test]
    fn chases_happen_and_only_from_inside_the_big_disk() {
        let mut field = TrainingField::default();
        let mut chase_steps = 0;
        for _ in 0..60 * 60 {
            field.update(1.0 / 60.0);
            for m in field.monsters() {
                if let Some(i) = m.target {
                    chase_steps += 1;
                    let d = (field.team()[i].body.position - m.body.position).length();
                    // Chosen inside the big disk at the start of the step; both moved since.
                    assert!(d <= RANGED_REACH + 2.0 * RUN_SPEED / 60.0, "{d}");
                    assert!(m.gait != Gait::Walking);
                }
            }
        }
        assert!(chase_steps > 60, "{chase_steps}");
        assert!(field.view().lines.iter().any(|l| l.color == color::TARGET));
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
    /// run, each gait about a third of the time, never faster than its gait, and one that never
    /// chased never further than its idle radius from the spawn.
    #[test]
    fn monsters_change_gait_and_keep_near_their_spawn() {
        let mut field = TrainingField::default();
        let mut steps_in = [0usize; 3];
        let mut visited = [[false; 3]; DEFAULT_MONSTERS];
        let mut chased = [false; DEFAULT_MONSTERS];
        let mut out = [0.0f32; DEFAULT_MONSTERS];
        for _ in 0..120 * 60 {
            field.update(1.0 / 60.0);
            for (i, m) in field.monsters().iter().enumerate() {
                chased[i] |= m.target.is_some();
                out[i] = out[i].max((m.body.position - m.spawn).length());
                if chased[i] {
                    continue;
                }
                let g = Gait::ALL.iter().position(|&g| g == m.gait).unwrap();
                steps_in[g] += 1;
                visited[i][g] = true;
                assert!(
                    m.body.speed <= m.gait.speed() + 1e-3,
                    "{:?} at {}",
                    m.gait,
                    m.body.speed
                );
            }
        }
        let total = steps_in.iter().sum::<usize>() as f32;
        let shares = steps_in.map(|n| n as f32 / total);
        assert!(shares.iter().all(|&s| s > 0.25 && s < 0.42), "{shares:?}");
        let quiet: Vec<usize> = (0..DEFAULT_MONSTERS).filter(|&i| !chased[i]).collect();
        assert!(quiet.len() >= 5, "{}", quiet.len());
        for &i in &quiet {
            assert!(visited[i].iter().all(|&b| b), "monster {i}");
            assert!(
                out[i] <= IDLE_RADIUS - BODY_RADIUS,
                "monster {i}: {}",
                out[i]
            );
        }
    }
}
