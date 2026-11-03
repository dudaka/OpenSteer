//! "Training Field": a Silkroad training spot in miniature, built to work on the lurer. A team
//! of six trains round one centre, each role keeping to its own radius: three dps (10), an HP
//! and an MP buffer (5) and our lurer (50); monsters spawn at random over a rectangular map.
//! Built so far (PLAN.md, Phase 8): the map, the disks, the spawns, and bodies at human speeds,
//! each roaming its own disk from one random spot to the next: the players always run, a
//! monster stands, walks or runs for a few seconds at a time. Every entity carries two disks
//! on its own position: a big one, its ranged reach (a player's 8 is its attack and its buffs,
//! a monster's 6 is its alarm), and a small one, its melee reach. Players are ranged; a
//! monster is melee (its rarity scaling its HP, as the game client does, and its bite, our
//! assumption), and chases the nearest player inside its big disk until that player is
//! inside its small disk, and keeps chasing while that player stays in its line of sight (a
//! third, biggest disk), forgetting it beyond. Everyone has HP (players MP too), a player's bars regenerating a point
//! a second. A monster bites the player it closes on once inside melee reach, and chases
//! whoever has hurt it most (its threat table) before anyone merely near. A dead player is back
//! at the centre after 5 s; a dead monster is replaced elsewhere after 3 s. A dps fires at the
//! nearest monster inside the dps disk, an area blast that hurts every monster round the
//! target and raises their threat. The buffers keep the team's bars up: each casts a 10 s buff
//! that recovers HP or MP on the member with the lowest bar under the threshold. The lurer tags
//! any monster in its reach it has not tagged yet, a scratch that puts the lurer on the
//! monster's threat table so it follows. The lurer's trips (`Lure`, caravan's D105 design):
//! wait 2 s at the centre until the training disk is quiet, run out to the nearest untagged
//! monster in the lurer's disk, keep tagging the next nearest, run back when enough chase
//! (or HP is low, or none is left), and count what the trips deliver.

use std::collections::HashMap;
use std::f32::consts::TAU;

use steer_core::containment::contain;
use steer_core::wall::Wall;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle_view, color, label, legend, line, vehicle_view};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

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
/// HP and MP: 100 points each (a point is a percent); a player regains `REGEN` of each a second,
/// a monster regains nothing.
pub const MAX_HP: f32 = 100.0;
pub const MAX_MP: f32 = 100.0;
pub const REGEN: f32 = 1.0;
/// The lurer's tag: at the nearest monster within its ranged reach not yet tagged by it, every
/// `TAG_COOLDOWN` for `TAG_MP`, `TAG_DAMAGE` off it (threat, not harm).
pub const TAG_DAMAGE: f32 = 1.0;
pub const TAG_MP: f32 = 2.0;
pub const TAG_COOLDOWN: f32 = 1.0;
/// The lurer's area skills, each a tag on up to `AOE_TARGETS` monsters: a blast of
/// `LURE_BLAST` round a monster within its ranged reach, and a pierce `PIERCE_WIDTH` either
/// side of a line `PIERCE_LENGTH` long toward a monster. It casts whichever catches the most
/// untagged monsters, the normal attack when neither would catch two.
pub const LURE_BLAST: f32 = 3.0;
pub const BLAST_MP: f32 = 6.0;
pub const BLAST_COOLDOWN: f32 = 4.0;
pub const PIERCE_LENGTH: f32 = 12.0;
pub const PIERCE_WIDTH: f32 = 1.5;
pub const PIERCE_MP: f32 = 8.0;
pub const PIERCE_COOLDOWN: f32 = 6.0;
/// Any area attack hits at most this many monsters, the nearest to its centre or line.
pub const AOE_TARGETS: usize = 5;
/// A dps's area attack: at a monster inside the dps disk and its ranged reach, every
/// `AOE_COOLDOWN` for `AOE_MP`, `AOE_DAMAGE` off every monster within `AOE_BLAST` of the target.
pub const AOE_DAMAGE: f32 = 10.0;
pub const AOE_BLAST: f32 = 2.0;
pub const AOE_MP: f32 = 6.0;
pub const AOE_COOLDOWN: f32 = 1.5;
/// The buffs: a buffer casts once a second, within its ranged reach, on the living member
/// (itself included) with the lowest bar under `BUFF_THRESHOLD` and no such buff running; the
/// buff recovers `HP_BUFF_RATE` HP or `MP_BUFF_RATE` MP a second for `BUFF_DURATION`.
pub const HP_BUFF_RATE: f32 = 4.0;
pub const MP_BUFF_RATE: f32 = 3.0;
pub const HP_BUFF_MP: f32 = 4.0;
pub const MP_BUFF_MP: f32 = 2.0;
pub const BUFF_DURATION: f32 = 10.0;
pub const BUFF_COOLDOWN: f32 = 1.0;
pub const BUFF_THRESHOLD: f32 = 0.9;
/// A running buff is shown as a ring this far out round the body (HP inside, MP outside).
const HP_BUFF_RING: f32 = 1.3;
const MP_BUFF_RING: f32 = 1.5;
/// The spawn mix by rarity, in percent (party giants off: this team cannot kill one).
const SPAWN_MIX: [(Rarity, u32); 6] = [
    (Rarity::Normal, 80),
    (Rarity::Champion, 12),
    (Rarity::Giant, 3),
    (Rarity::PartyNormal, 4),
    (Rarity::PartyChampion, 1),
    (Rarity::PartyGiant, 0),
];
/// A monster's bite: `BITE` HP off its target every `BITE_COOLDOWN` while inside melee reach,
/// times its rarity's damage multiplier.
pub const BITE: f32 = 3.0;
pub const BITE_COOLDOWN: f32 = 1.5;
/// The lurer's strategy: leave the centre as soon as its HP is back (`LEAVE_HP`), with at most
/// the load cap's monsters in the dps disk (0: no cap); come back with N (`Settings::chasers`)
/// chasing, HP under `BACK_HP`, or no untagged monster left in the lurer's disk; home is
/// within `HOME` of the centre.
pub const BACK_HP: f32 = 0.4;
pub const HOME: f32 = 1.0;
/// The lurer leaves the centre once its HP is back to this share (the user: 90%, to cut the
/// rest).
pub const LEAVE_HP: f32 = 0.9;
/// The tour planner: the N stops (untagged monsters in the lurer's disk) and their order that
/// make the shortest run from the lurer through them and home, solved exactly over the
/// `TOUR_CANDIDATES` nearest and planned again every `PLAN_PERIOD`.
pub const TOUR_CANDIDATES: usize = 8;
pub const PLAN_PERIOD: f32 = 1.0;
/// The balance sliders and their defaults: N, the monsters' running speed (faster than the
/// players' 5, as in the game), the bite's damage and reach, the load cap.
const CHASERS: Range = Range::new(1.0, 10.0, 1.0);
const MONSTER_SPEED: Range = Range::new(3.0, 8.0, 0.5);
const BITE_RANGE: Range = Range::new(1.0, 10.0, 0.5);
const BITE_REACH: Range = Range::new(1.0, 5.0, 0.5);
const LOAD_CAP: Range = Range::new(0.0, 10.0, 1.0);
pub const BACK_CHASERS: usize = 3;
pub const DEFAULT_MONSTER_SPEED: f32 = 6.0;
pub const DEFAULT_LOAD_CAP: usize = 3;

/// Which planner lays out a trip: `Stops` visits N monsters one by one (the shortest such
/// tour); `Casts` plans casts, a blast or pierce tagging a whole cluster at one stop, until N
/// are tagged (the shortest such run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Planner {
    Stops,
    Casts,
}

/// A stop on the lurer's tour: get within `reach` of monster `id` and cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub id: u32,
    pub reach: f32,
}

/// The slider settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub planner: Planner,
    /// N: the lurer turns back with this many chasers.
    pub chasers: usize,
    pub monster_speed: f32,
    pub bite: f32,
    pub bite_reach: f32,
    /// The lurer stays while the dps disk holds more than this; 0 for no cap.
    pub load_cap: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            planner: Planner::Casts,
            chasers: BACK_CHASERS,
            monster_speed: DEFAULT_MONSTER_SPEED,
            bite: BITE,
            bite_reach: MELEE_REACH,
            load_cap: DEFAULT_LOAD_CAP,
        }
    }
}
/// A dead player is back at the centre with full bars after this long; a dead monster's
/// replacement spawns after this long.
pub const PLAYER_RESPAWN: f32 = 5.0;
pub const MONSTER_RESPAWN: f32 = 3.0;
/// Bars over a body: twice the body's width, this far above its centre (up the screen is -y),
/// each `BAR_STRANDS` lines `BAR_STRAND_GAP` apart, since a WebGL line is always 1 px.
const BAR_WIDTH: f32 = 4.0 * BODY_RADIUS;
const HP_BAR_LIFT: f32 = 1.8;
const MP_BAR_LIFT: f32 = 2.5;
const BAR_STRANDS: usize = 3;
const BAR_STRAND_GAP: f32 = 0.12;
/// Every entity's two reaches, centre to centre: the big disk (a player's ranged attack and
/// buffs; a monster's alarm, smaller) and the small disk (melee; a monster's attack).
pub const RANGED_REACH: f32 = 8.0;
pub const ALARM_REACH: f32 = 6.0;
pub const MELEE_REACH: f32 = 2.0;
/// A chasing monster's line of sight (the user: 30-40): it keeps its target inside, forgets
/// it beyond.
pub const LOS_REACH: f32 = 35.0;
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
    pub hp: f32,
    pub mp: f32,
    /// The spot it is running to inside its disk.
    spot: Vec2,
    /// Dead until this time (simulation time), then back at the centre.
    dead_until: Option<f32>,
    /// A dps's target: the monster (an index into the monsters) it closes on or fires at.
    pub target: Option<usize>,
    /// When it may use its skill next (a dps's attack, a buffer's buff).
    skill_ready: f32,
    /// The HP and MP buffs on it run until these times.
    pub hp_buff_until: f32,
    pub mp_buff_until: f32,
    /// The lurer's area skills' cooldowns.
    blast_ready: f32,
    pierce_ready: f32,
}

impl Player {
    pub fn is_dead(&self) -> bool {
        self.dead_until.is_some()
    }
}

/// A monster's rarity. HP multipliers are the game client's (caravan read them out of
/// sro_client.exe 37: champion x2, giant x20, a party monster x10 more). Damage multipliers
/// are our assumption (champion x1.5, giant x3, the party flag x2 on top); caravan has no
/// verified numbers for them. Body size is ours too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rarity {
    Normal,
    Champion,
    Giant,
    PartyNormal,
    PartyChampion,
    PartyGiant,
}

impl Rarity {
    pub fn hp_multiplier(self) -> f32 {
        match self {
            Rarity::Normal => 1.0,
            Rarity::Champion => 2.0,
            Rarity::Giant => 20.0,
            Rarity::PartyNormal => 10.0,
            Rarity::PartyChampion => 20.0,
            Rarity::PartyGiant => 200.0,
        }
    }

    pub fn damage_multiplier(self) -> f32 {
        match self {
            Rarity::Normal => 1.0,
            Rarity::Champion => 1.5,
            Rarity::Giant => 3.0,
            Rarity::PartyNormal => 2.0,
            Rarity::PartyChampion => 3.0,
            Rarity::PartyGiant => 6.0,
        }
    }

    /// The body's radius: a champion a quarter bigger, a giant twice.
    pub fn body_radius(self) -> f32 {
        match self {
            Rarity::Normal | Rarity::PartyNormal => BODY_RADIUS,
            Rarity::Champion | Rarity::PartyChampion => 1.25 * BODY_RADIUS,
            Rarity::Giant | Rarity::PartyGiant => 2.0 * BODY_RADIUS,
        }
    }

    /// The label over a monster; none for a normal one.
    pub fn name(self) -> Option<&'static str> {
        match self {
            Rarity::Normal => None,
            Rarity::Champion => Some("champion"),
            Rarity::Giant => Some("giant"),
            Rarity::PartyNormal => Some("party"),
            Rarity::PartyChampion => Some("party champion"),
            Rarity::PartyGiant => Some("party giant"),
        }
    }
}

/// A rarity drawn by `SPAWN_MIX`.
fn draw_rarity(rng: &mut fastrand::Rng) -> Rarity {
    let total: u32 = SPAWN_MIX.iter().map(|&(_, w)| w).sum();
    let mut roll = rng.u32(..total);
    for &(rarity, weight) in &SPAWN_MIX {
        if roll < weight {
            return rarity;
        }
        roll -= weight;
    }
    Rarity::Normal
}

#[derive(Debug, Clone)]
pub struct Monster {
    /// Stable through deaths and respawns (indices are not): the tour names stops by it.
    pub id: u32,
    pub rarity: Rarity,
    /// `MAX_HP` times the rarity's multiplier.
    pub max_hp: f32,
    pub gait: Gait,
    /// When the next gait is drawn (simulation time).
    until: f32,
    pub body: Vehicle,
    pub hp: f32,
    /// Where it appeared: it idles round here and, its target lost, walks back here.
    pub spawn: Vec2,
    /// The spot it is going to inside its idle disk.
    spot: Vec2,
    /// The player (an index into the team) it is closing on: the top of its threat table,
    /// else the nearest inside its alarm disk.
    pub target: Option<usize>,
    /// Damage each player has dealt it.
    pub threat: HashMap<usize, f32>,
    /// Metrics: it has been counted as delivered (reached the dps disk chasing the lurer), and
    /// whether it was chasing the lurer after the last step.
    delivered: bool,
    was_chasing_lurer: bool,
    /// When it may bite next.
    bite_ready: f32,
}

/// Where the lurer is in its trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LureStage {
    #[default]
    Centre,
    Out,
    Back,
}

/// The lurer's trip state and the metrics the trips are judged by.
#[derive(Debug, Clone, Default)]
pub struct Lure {
    pub stage: LureStage,
    pub trips: u32,
    /// Monsters that reached the dps disk while chasing the lurer.
    pub delivered: u32,
    /// Monsters that stopped chasing the lurer outside the dps disk (out of sight, or taken
    /// over by someone's damage).
    pub lost: u32,
    /// Steps the lurer spent inside the dps disk, of all steps.
    pub steps_in_disk: u32,
    pub steps: u32,
    /// Casts: normal attacks, blasts and pierces.
    pub tags: u32,
    pub blasts: u32,
    pub pierces: u32,
    /// The planned stops (monster ids) of the current trip, next first, and when they were
    /// planned.
    pub tour: Vec<Stop>,
    planned_at: f32,
    /// Session metrics: when the current trip left, the trips completed, their total time,
    /// the HP the lurer had back home (total and lowest), and the distance it walked.
    left_at: f32,
    pub sessions: u32,
    pub session_time: f32,
    pub hp_back_total: f32,
    pub hp_back_min: f32,
    pub walked: f32,
}

impl Lure {
    pub fn share_in_disk(&self) -> f32 {
        self.steps_in_disk as f32 / self.steps.max(1) as f32
    }

    pub fn mean_session(&self) -> f32 {
        self.session_time / self.sessions.max(1) as f32
    }

    pub fn mean_hp_back(&self) -> f32 {
        self.hp_back_total / self.sessions.max(1) as f32
    }
}

/// The cheapest tour of `n` of `stops` (id, position): the order that minimises the run from
/// `from` through them to `home`, found by an exact search with pruning (at most 8 stops).
pub fn plan_tour(from: Vec2, home: Vec2, stops: &[(u32, Vec2)], n: usize) -> Vec<u32> {
    fn search(
        at: Vec2,
        home: Vec2,
        stops: &[(u32, Vec2)],
        left: usize,
        used: &mut Vec<usize>,
        cost: f32,
        best: &mut (f32, Vec<usize>),
    ) {
        if left == 0 {
            let total = cost + (home - at).length();
            if total < best.0 {
                *best = (total, used.clone());
            }
            return;
        }
        for (i, &(_, p)) in stops.iter().enumerate() {
            if used.contains(&i) {
                continue;
            }
            let next = cost + (p - at).length();
            if next >= best.0 {
                continue;
            }
            used.push(i);
            search(p, home, stops, left - 1, used, next, best);
            used.pop();
        }
    }
    let n = n.min(stops.len());
    let mut best = (f32::MAX, Vec::new());
    search(from, home, stops, n, &mut Vec::new(), 0.0, &mut best);
    best.1.into_iter().map(|i| stops[i].0).collect()
}

#[derive(Debug, Clone)]
pub struct TrainingField {
    /// The lurer, three dps, the HP buffer, the MP buffer.
    team: Vec<Player>,
    monsters: Vec<Monster>,
    /// How many monsters the map keeps alive (the slider).
    wanted: usize,
    /// Times at which a killed monster's replacement may spawn.
    respawns: Vec<f32>,
    pub kills: u32,
    next_id: u32,
    pub deaths: u32,
    pub lure: Lure,
    pub settings: Settings,
    rng: fastrand::Rng,
    time: f32,
}

impl Monster {
    /// Makes it a `rarity` monster at full HP, its body sized to match.
    pub fn set_rarity(&mut self, rarity: Rarity) {
        self.rarity = rarity;
        self.max_hp = MAX_HP * rarity.hp_multiplier();
        self.hp = self.max_hp;
        self.body.radius = rarity.body_radius();
    }

    /// Idle at home: inside its idle disk with nobody to chase and nobody who hurt it.
    fn at_home(&self) -> bool {
        self.target.is_none()
            && self.threat.is_empty()
            && (self.body.position - self.spawn).length() <= IDLE_RADIUS
    }
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

/// A bar over the body at `at`: `BAR_WIDTH` wide at full, `lift` above the centre, filled
/// from the left to `fill` (0 to 1) over a full-width `color::TRACK` track, so the empty part
/// shows; `BAR_STRANDS` lines stacked for thickness, the track's drawn first (under).
fn bar(at: Vec2, lift: f32, fill: f32, color: u32) -> impl Iterator<Item = LineView> {
    let strands = move |fill: f32, color: u32| {
        (0..BAR_STRANDS).map(move |i| {
            let left = at + Vec2::new(-BAR_WIDTH / 2.0, -lift - i as f32 * BAR_STRAND_GAP);
            line(left, left + Vec2::new(BAR_WIDTH * fill, 0.0), color)
        })
    };
    strands(1.0, color::TRACK).chain(strands(fill, color))
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
    run_speed: f32,
    spot: &mut Vec2,
    (centre, radius): (Vec2, f32),
    dt: f32,
    rng: &mut fastrand::Rng,
) {
    let room = radius - BODY_RADIUS;
    body.max_speed = speed_of(gait, run_speed);
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

/// A gait's speed with `run_speed` as the run (players run at `RUN_SPEED`, monsters at the
/// slider's).
fn speed_of(gait: Gait, run_speed: f32) -> f32 {
    match gait {
        Gait::Running => run_speed,
        _ => gait.speed(),
    }
}

/// One step of a body closing on `target`: a run toward it while it is further than `reach`,
/// a stop (facing it) once inside.
fn hunt(body: &mut Vehicle, target: Vec2, reach: f32, run_speed: f32, dt: f32) -> Gait {
    let gait = if (target - body.position).length() > reach {
        Gait::Running
    } else {
        Gait::Standing
    };
    body.max_speed = speed_of(gait, run_speed);
    let force = match gait {
        Gait::Running => steer::seek(body, target),
        _ => Vec2::ZERO,
    };
    body.apply_steering_force(force, dt);
    gait
}

/// The player a monster chases: the living one that has dealt it the most damage (the lowest
/// index on a tie), else `None`.
fn threat_target(threat: &HashMap<usize, f32>, alive: &[bool]) -> Option<usize> {
    (threat.iter())
        .filter(|(&i, _)| alive[i])
        .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(&i, _)| i)
}

/// The monster nearest to `at` within the lurer's ranged reach that player `lurer` has not
/// tagged yet (no threat of theirs on it): the lurer's next tag.
fn untagged_in_reach(lurer: usize, at: Vec2, monsters: &[Monster]) -> Option<usize> {
    (monsters.iter().enumerate())
        .filter(|(_, m)| !m.threat.contains_key(&lurer))
        .map(|(i, m)| (i, (m.body.position - at).length()))
        .filter(|&(_, d)| d <= RANGED_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The cheapest run that tags at least `n` monsters by casts, from `from` through its stops
/// and home: an exact search over `candidates` (monster ids). At each stop the lurer gets
/// within a skill's reach of a monster and casts: the normal attack tags it alone (reach 8),
/// the blast every untagged monster within `LURE_BLAST` of it (reach 8), the pierce every
/// untagged monster on the line from the cast point to it (reach `PIERCE_LENGTH`); the blast
/// and pierce only when `skills` (blast ready, pierce ready) allow. The search ends a run
/// once `n` are tagged or nothing more can be, and keeps the shortest, the fewest casts on a
/// tie.
pub fn plan_casts(
    from: Vec2,
    home: Vec2,
    lurer: usize,
    monsters: &[Monster],
    candidates: &[u32],
    n: usize,
    skills: (bool, bool),
) -> Vec<Stop> {
    struct Search<'a> {
        home: Vec2,
        lurer: usize,
        monsters: &'a [Monster],
        candidates: &'a [u32],
        n: usize,
        skills: (bool, bool),
        best: (f32, Vec<Stop>),
    }
    impl Search<'_> {
        fn untagged(&self, hits: Vec<usize>, tagged: &[u32]) -> Vec<u32> {
            (hits.into_iter().map(|i| &self.monsters[i]))
                .filter(|m| !m.threat.contains_key(&self.lurer) && !tagged.contains(&m.id))
                .map(|m| m.id)
                .collect()
        }

        fn go(&mut self, at: Vec2, tagged: &mut Vec<u32>, cost: f32, plan: &mut Vec<Stop>) {
            let mut expanded = false;
            if tagged.len() < self.n {
                for &id in self.candidates {
                    if tagged.contains(&id) {
                        continue;
                    }
                    let Some(m) = self.monsters.iter().find(|m| m.id == id) else {
                        continue;
                    };
                    let target = m.body.position;
                    let d = (target - at).length();
                    let dir = if d > 0.0 {
                        (target - at) / d
                    } else {
                        Vec2::ZERO
                    };
                    let cast_at = |reach: f32| at + dir * (d - reach).max(0.0);
                    // The normal attack always; a blast or pierce only when it tags two or
                    // more, the rule `lure_cast` fires them by.
                    let mut options: Vec<(f32, Vec<u32>)> = vec![(RANGED_REACH, vec![id])];
                    if self.skills.0 {
                        let hits = in_blast(target, LURE_BLAST, self.monsters);
                        let tags = self.untagged(hits, tagged);
                        if tags.len() >= 2 {
                            options.push((RANGED_REACH, tags));
                        }
                    }
                    if self.skills.1 {
                        let hits = along_pierce(cast_at(PIERCE_LENGTH), target, self.monsters);
                        let tags = self.untagged(hits, tagged);
                        if tags.len() >= 2 {
                            options.push((PIERCE_LENGTH, tags));
                        }
                    }
                    for (reach, tags) in options {
                        if tags.is_empty() {
                            continue;
                        }
                        // A node with any way on is no end of a run, pruned or not.
                        expanded = true;
                        let next = cost + (d - reach).max(0.0);
                        if next >= self.best.0 {
                            continue;
                        }
                        let before = tagged.len();
                        tagged.extend(&tags);
                        plan.push(Stop { id, reach });
                        self.go(cast_at(reach), tagged, next, plan);
                        plan.pop();
                        tagged.truncate(before);
                    }
                }
            }
            if !expanded && !plan.is_empty() {
                // The shortest run; on a tie (a straight row, say) the fewer casts.
                let total = cost + (self.home - at).length();
                let shorter =
                    total < self.best.0 || (total == self.best.0 && plan.len() < self.best.1.len());
                if shorter {
                    self.best = (total, plan.clone());
                }
            }
        }
    }
    let mut search = Search {
        home,
        lurer,
        monsters,
        candidates,
        n,
        skills,
        best: (f32::MAX, Vec::new()),
    };
    search.go(from, &mut Vec::new(), 0.0, &mut Vec::new());
    search.best.1
}

/// The planner's candidates: the monsters in the lurer's disk that the lurer has not tagged,
/// the `TOUR_CANDIDATES` nearest to `at`, as (id, position).
fn tour_candidates(lurer: usize, at: Vec2, monsters: &[Monster]) -> Vec<(u32, Vec2)> {
    let mut near: Vec<&Monster> = (monsters.iter())
        .filter(|m| !m.threat.contains_key(&lurer))
        .filter(|m| (m.body.position - CENTRE).length() <= LURE_RADIUS)
        .collect();
    near.sort_by(|a, b| {
        let d = |m: &Monster| (m.body.position - at).length();
        d(a).total_cmp(&d(b))
    });
    near.truncate(TOUR_CANDIDATES);
    near.into_iter().map(|m| (m.id, m.body.position)).collect()
}

/// One step of the lurer's trip. At the centre it stands until its HP is back to `LEAVE_HP`
/// and the dps disk holds at most the load cap's monsters; out, it follows its tour (planned
/// again every `PLAN_PERIOD`, stops already tagged dropped; the tag itself fires from
/// `lure_cast`) until N chase, its HP is under `BACK_HP` or no stop is left; back, it runs
/// home, and on arrival the session's time and HP are recorded.
fn lure_step(
    p: &mut Player,
    lure: &mut Lure,
    settings: &Settings,
    monsters: &[Monster],
    chasers: usize,
    load: usize,
    (now, dt): (f32, f32),
) {
    let before = p.body.position;
    match lure.stage {
        LureStage::Centre => {
            hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt);
            let quiet = settings.load_cap == 0 || load <= settings.load_cap;
            if p.hp >= LEAVE_HP * MAX_HP && quiet {
                lure.stage = LureStage::Out;
                lure.trips += 1;
                lure.left_at = now;
                lure.planned_at = f32::MIN;
            }
        }
        LureStage::Out => {
            let back = chasers >= settings.chasers || p.hp < BACK_HP * MAX_HP;
            let at = p.body.position;
            let find = |id: u32| monsters.iter().find(|m| m.id == id);
            lure.tour
                .retain(|s| find(s.id).is_some_and(|m| !m.threat.contains_key(&0)));
            if !back && (lure.tour.is_empty() || now - lure.planned_at >= PLAN_PERIOD) {
                let candidates = tour_candidates(0, at, monsters);
                let n = settings.chasers;
                lure.tour = match settings.planner {
                    Planner::Stops => (plan_tour(at, CENTRE, &candidates, n).into_iter())
                        .map(|id| Stop {
                            id,
                            reach: RANGED_REACH,
                        })
                        .collect(),
                    Planner::Casts => {
                        let ids: Vec<u32> = candidates.iter().map(|c| c.0).collect();
                        let skills = (now >= p.blast_ready, now >= p.pierce_ready);
                        plan_casts(at, CENTRE, 0, monsters, &ids, n, skills)
                    }
                };
                lure.planned_at = now;
            }
            let next = lure.tour.first().map(|s| (find(s.id), s.reach));
            match next {
                Some((Some(m), reach)) if !back => {
                    hunt(&mut p.body, m.body.position, reach, RUN_SPEED, dt);
                }
                _ => {
                    lure.stage = LureStage::Back;
                    lure.tour.clear();
                    hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt);
                }
            }
        }
        LureStage::Back => {
            if hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt) == Gait::Standing {
                lure.stage = LureStage::Centre;
                lure.sessions += 1;
                lure.session_time += now - lure.left_at;
                lure.hp_back_total += p.hp;
                lure.hp_back_min = if lure.sessions == 1 {
                    p.hp
                } else {
                    lure.hp_back_min.min(p.hp)
                };
            }
        }
    }
    lure.walked += (p.body.position - before).length();
}

/// The monsters within `radius` of `at`, nearest first, at most `AOE_TARGETS`.
fn in_blast(at: Vec2, radius: f32, monsters: &[Monster]) -> Vec<usize> {
    let mut hit: Vec<(usize, f32)> = (monsters.iter().enumerate())
        .map(|(i, m)| (i, (m.body.position - at).length()))
        .filter(|&(_, d)| d <= radius)
        .collect();
    hit.sort_by(|a, b| a.1.total_cmp(&b.1));
    hit.truncate(AOE_TARGETS);
    hit.into_iter().map(|(i, _)| i).collect()
}

/// The monsters within `PIERCE_WIDTH` of the line from `from` toward `toward`,
/// `PIERCE_LENGTH` long, nearest first, at most `AOE_TARGETS`.
fn along_pierce(from: Vec2, toward: Vec2, monsters: &[Monster]) -> Vec<usize> {
    let dir = (toward - from).normalize();
    let mut hit: Vec<(usize, f32)> = (monsters.iter().enumerate())
        .filter_map(|(i, m)| {
            let off = m.body.position - from;
            let along = off.dot(dir);
            let aside = (off - dir * along).length();
            ((0.0..=PIERCE_LENGTH).contains(&along) && aside <= PIERCE_WIDTH).then_some((i, along))
        })
        .collect();
    hit.sort_by(|a, b| a.1.total_cmp(&b.1));
    hit.truncate(AOE_TARGETS);
    hit.into_iter().map(|(i, _)| i).collect()
}

/// The lurer's cast this step, if any: the blast or the pierce that catches the most monsters
/// it has not tagged (two at least), else the normal attack on `target`; each only when
/// ready and affordable. Returns the monsters hit; MP, cooldowns and counts are updated.
fn lure_cast(
    p: &mut Player,
    lurer: usize,
    target: Option<usize>,
    monsters: &[Monster],
    counts: &mut Lure,
    now: f32,
) -> Vec<usize> {
    let untagged = |hits: &[usize]| {
        hits.iter()
            .filter(|&&i| !monsters[i].threat.contains_key(&lurer))
            .count()
    };
    let at = p.body.position;
    let candidates = (monsters.iter().enumerate())
        .filter(|(_, m)| !m.threat.contains_key(&lurer))
        .map(|(i, m)| (i, (m.body.position - at).length()));
    let (mut blast, mut pierce): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (i, d) in candidates {
        let centre = monsters[i].body.position;
        if d <= RANGED_REACH {
            let hits = in_blast(centre, LURE_BLAST, monsters);
            if untagged(&hits) > untagged(&blast) {
                blast = hits;
            }
        }
        if d <= PIERCE_LENGTH {
            let hits = along_pierce(at, centre, monsters);
            if untagged(&hits) > untagged(&pierce) {
                pierce = hits;
            }
        }
    }
    let blast_ok = now >= p.blast_ready && p.mp >= BLAST_MP && untagged(&blast) >= 2;
    let pierce_ok = now >= p.pierce_ready && p.mp >= PIERCE_MP && untagged(&pierce) >= 2;
    if pierce_ok && (!blast_ok || untagged(&pierce) > untagged(&blast)) {
        p.mp -= PIERCE_MP;
        p.pierce_ready = now + PIERCE_COOLDOWN;
        counts.pierces += 1;
        return pierce;
    }
    if blast_ok {
        p.mp -= BLAST_MP;
        p.blast_ready = now + BLAST_COOLDOWN;
        counts.blasts += 1;
        return blast;
    }
    match target {
        Some(t) if now >= p.skill_ready && p.mp >= TAG_MP => {
            p.mp -= TAG_MP;
            p.skill_ready = now + TAG_COOLDOWN;
            counts.tags += 1;
            vec![t]
        }
        _ => Vec::new(),
    }
}

/// The monster inside the dps disk nearest to `at`: a dps's target.
fn nearest_monster_in_disk(at: Vec2, monsters: &[Monster]) -> Option<usize> {
    (monsters.iter().enumerate())
        .filter(|(_, m)| (m.body.position - CENTRE).length() <= DPS_RADIUS)
        .min_by(|a, b| {
            let d = |m: &Monster| (m.body.position - at).length();
            d(a.1).total_cmp(&d(b.1))
        })
        .map(|(i, _)| i)
}

/// The nearest of `players` within `ALARM_REACH` of `at`, and its distance.
fn player_in_reach(at: Vec2, players: &[Vec2]) -> Option<(usize, f32)> {
    (players.iter().enumerate())
        .map(|(i, &p)| (i, (p - at).length()))
        .filter(|&(_, d)| d <= ALARM_REACH)
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
            hp: MAX_HP,
            mp: MAX_MP,
            spot: at,
            dead_until: None,
            target: None,
            skill_ready: 0.0,
            hp_buff_until: 0.0,
            mp_buff_until: 0.0,
            blast_ready: 0.0,
            pierce_ready: 0.0,
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
            respawns: Vec::new(),
            kills: 0,
            next_id: 0,
            deaths: 0,
            lure: Lure::default(),
            settings: Settings::default(),
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
        self.bury();
        self.spawn_missing();
        let rng = &mut self.rng;
        let chasers = self.monsters.iter().filter(|m| m.target == Some(0)).count();
        let load = (self.monsters.iter())
            .filter(|m| (m.body.position - CENTRE).length() <= DPS_RADIUS)
            .count();
        let mut hits = Vec::new();
        let mut tags = Vec::new();
        for (i, p) in self.team.iter_mut().enumerate() {
            if let Some(until) = p.dead_until {
                if self.time >= until {
                    *p = Player {
                        body: standing(CENTRE),
                        hp: MAX_HP,
                        mp: MAX_MP,
                        spot: CENTRE,
                        dead_until: None,
                        target: None,
                        skill_ready: self.time,
                        hp_buff_until: 0.0,
                        mp_buff_until: 0.0,
                        blast_ready: self.time,
                        pierce_ready: self.time,
                        role: p.role,
                    };
                }
                continue;
            }
            let hp_rate = REGEN
                + if self.time < p.hp_buff_until {
                    HP_BUFF_RATE
                } else {
                    0.0
                };
            let mp_rate = REGEN
                + if self.time < p.mp_buff_until {
                    MP_BUFF_RATE
                } else {
                    0.0
                };
            p.hp = (p.hp + hp_rate * dt).min(MAX_HP);
            p.mp = (p.mp + mp_rate * dt).min(MAX_MP);
            p.target = match p.role {
                Role::Dps => nearest_monster_in_disk(p.body.position, &self.monsters),
                Role::Lurer => untagged_in_reach(i, p.body.position, &self.monsters),
                _ => None,
            };
            if p.role == Role::Lurer {
                let now = self.time;
                let hit = lure_cast(p, i, p.target, &self.monsters, &mut self.lure, now);
                tags.extend(hit.into_iter().map(|t| (i, t)));
            }
            if p.role == Role::Lurer {
                let (settings, now) = (&self.settings, self.time);
                let lure = &mut self.lure;
                lure_step(p, lure, settings, &self.monsters, chasers, load, (now, dt));
                continue;
            }
            let Some(t) = p.target else {
                let r = p.role.radius();
                roam(
                    &mut p.body,
                    Gait::Running,
                    RUN_SPEED,
                    &mut p.spot,
                    (CENTRE, r),
                    dt,
                    rng,
                );
                continue;
            };
            let at = self.monsters[t].body.position;
            if hunt(&mut p.body, at, RANGED_REACH, RUN_SPEED, dt) == Gait::Standing
                && self.time >= p.skill_ready
                && p.mp >= AOE_MP
            {
                p.mp -= AOE_MP;
                p.skill_ready = self.time + AOE_COOLDOWN;
                hits.push((i, at));
            }
        }
        for b in 0..self.team.len() {
            self.cast_buff(b);
        }
        let rng = &mut self.rng;
        for (lurer, t) in tags {
            let m = &mut self.monsters[t];
            m.hp -= TAG_DAMAGE;
            *m.threat.entry(lurer).or_insert(0.0) += TAG_DAMAGE;
        }
        for (shooter, at) in hits {
            for t in in_blast(at, AOE_BLAST, &self.monsters) {
                let m = &mut self.monsters[t];
                m.hp -= AOE_DAMAGE;
                *m.threat.entry(shooter).or_insert(0.0) += AOE_DAMAGE;
            }
        }
        let alive: Vec<bool> = self.team.iter().map(|p| !p.is_dead()).collect();
        let players: Vec<Vec2> = self.team.iter().map(|p| p.body.position).collect();
        let living: Vec<Vec2> = (players.iter().zip(&alive))
            .map(|(&at, &a)| if a { at } else { Vec2::new(f32::MAX, f32::MAX) })
            .collect();
        for m in &mut self.monsters {
            let at = m.body.position;
            let in_sight = |i: usize| alive[i] && (players[i] - at).length() <= LOS_REACH;
            // Beyond its line of sight a player is forgotten: threat and chase alike.
            m.threat.retain(|&i, _| in_sight(i));
            let kept = m.target.filter(|&i| in_sight(i));
            m.target = threat_target(&m.threat, &alive)
                .or(kept)
                .or_else(|| player_in_reach(at, &living).map(|(i, _)| i));
            if let Some(i) = m.target {
                let (reach, speed) = (self.settings.bite_reach, self.settings.monster_speed);
                m.gait = hunt(&mut m.body, players[i], reach, speed, dt);
                // A fresh gait draw as soon as the chase ends.
                m.until = self.time;
                let in_reach = (players[i] - m.body.position).length() <= reach;
                if in_reach && self.time >= m.bite_ready {
                    m.bite_ready = self.time + BITE_COOLDOWN;
                    self.team[i].hp -= self.settings.bite * m.rarity.damage_multiplier();
                }
                continue;
            }
            // Back home with nobody after it, a monster is whole again (the game resets a
            // leashed monster's HP).
            if m.at_home() {
                m.hp = m.max_hp;
            }
            if self.time >= m.until {
                m.gait = Gait::ALL[rng.usize(..Gait::ALL.len())];
                m.until = self.time + GAIT_MIN_S + (GAIT_MAX_S - GAIT_MIN_S) * rng.f32();
            }
            let speed = self.settings.monster_speed;
            let disk = (m.spawn, IDLE_RADIUS);
            roam(&mut m.body, m.gait, speed, &mut m.spot, disk, dt, rng);
        }
        self.count_deliveries();
    }

    /// The trip metrics after a step: a monster chasing the lurer that enters the dps disk is
    /// delivered (once); one that stops chasing outside the disk is lost; the lurer's steps
    /// inside the disk are counted.
    fn count_deliveries(&mut self) {
        let lurer_dead = self.team[0].is_dead();
        for m in &mut self.monsters {
            let chasing = m.target == Some(0);
            let inside = (m.body.position - CENTRE).length() <= DPS_RADIUS;
            if chasing && inside && !m.delivered {
                m.delivered = true;
                self.lure.delivered += 1;
            }
            if m.was_chasing_lurer && !chasing && !inside && !lurer_dead {
                self.lure.lost += 1;
            }
            m.was_chasing_lurer = chasing;
        }
        self.lure.steps += 1;
        if (self.team[0].body.position - CENTRE).length() <= DPS_RADIUS {
            self.lure.steps_in_disk += 1;
        }
    }

    /// The planner buttons.
    pub fn action(&mut self, id: &str) {
        match id {
            "plan_casts" => self.settings.planner = Planner::Casts,
            "plan_stops" => self.settings.planner = Planner::Stops,
            _ => {}
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        let s = &mut self.settings;
        match id {
            "monsters" => {
                self.wanted = MONSTERS.clamp(value) as usize;
                self.monsters.truncate(self.wanted);
                self.spawn_missing();
            }
            "chasers" => s.chasers = CHASERS.clamp(value) as usize,
            "monster_speed" => s.monster_speed = MONSTER_SPEED.clamp(value),
            "bite" => s.bite = BITE_RANGE.clamp(value),
            "bite_reach" => s.bite_reach = BITE_REACH.clamp(value),
            "load_cap" => s.load_cap = LOAD_CAP.clamp(value) as usize,
            _ => {}
        }
    }

    /// Buffer `b` (nothing for other roles) casts its buff when ready and affordable: on the
    /// living member within its ranged reach with the lowest bar under the threshold and no
    /// such buff running, the buffer itself included.
    fn cast_buff(&mut self, b: usize) {
        type Read = fn(&Player) -> f32;
        let buffer = &self.team[b];
        let (cost, bar, until): (f32, Read, Read) = match buffer.role {
            Role::HpBuffer => (HP_BUFF_MP, |p| p.hp / MAX_HP, |p| p.hp_buff_until),
            Role::MpBuffer => (MP_BUFF_MP, |p| p.mp / MAX_MP, |p| p.mp_buff_until),
            _ => return,
        };
        if buffer.is_dead() || self.time < buffer.skill_ready || buffer.mp < cost {
            return;
        }
        let at = buffer.body.position;
        let member = (self.team.iter().enumerate())
            .filter(|(_, p)| !p.is_dead() && (p.body.position - at).length() <= RANGED_REACH)
            .filter(|(_, p)| bar(p) < BUFF_THRESHOLD && self.time >= until(p))
            .min_by(|a, b| bar(a.1).total_cmp(&bar(b.1)))
            .map(|(i, _)| i);
        let Some(i) = member else {
            return;
        };
        match self.team[b].role {
            Role::HpBuffer => self.team[i].hp_buff_until = self.time + BUFF_DURATION,
            _ => self.team[i].mp_buff_until = self.time + BUFF_DURATION,
        }
        self.team[b].mp -= cost;
        self.team[b].skill_ready = self.time + BUFF_COOLDOWN;
    }

    /// Deaths: a player at 0 HP dies (back after `PLAYER_RESPAWN`; every monster forgets it);
    /// a monster at 0 HP is removed and counted, its replacement due after `MONSTER_RESPAWN`.
    fn bury(&mut self) {
        for (i, p) in self.team.iter_mut().enumerate() {
            if p.hp <= 0.0 && !p.is_dead() {
                p.hp = 0.0;
                p.dead_until = Some(self.time + PLAYER_RESPAWN);
                self.deaths += 1;
                for m in &mut self.monsters {
                    m.threat.remove(&i);
                    if m.target == Some(i) {
                        m.target = None;
                    }
                }
            }
        }
        let before = self.monsters.len();
        self.monsters.retain(|m| m.hp > 0.0);
        let killed = before - self.monsters.len();
        self.kills += killed as u32;
        self.respawns
            .extend(std::iter::repeat_n(self.time + MONSTER_RESPAWN, killed));
    }

    /// Spawns monsters until `wanted` are on the map, standing until their first gait draw;
    /// a killed monster's slot waits for its respawn time.
    fn spawn_missing(&mut self) {
        let now = self.time;
        self.respawns.retain(|&t| t > now);
        while self.monsters.len() + self.respawns.len() < self.wanted {
            let at = self.spawn_point();
            let rarity = draw_rarity(&mut self.rng);
            self.next_id += 1;
            let mut monster = Monster {
                id: self.next_id,
                rarity,
                max_hp: MAX_HP,
                gait: Gait::Standing,
                until: self.time,
                body: standing(at),
                hp: MAX_HP,
                spawn: at,
                spot: at,
                target: None,
                threat: HashMap::new(),
                delivered: false,
                was_chasing_lurer: false,
                bite_ready: self.time,
            };
            monster.set_rarity(rarity);
            self.monsters.push(monster);
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
        for p in &self.team {
            let at = p.body.position;
            lines.extend(bar(at, HP_BAR_LIFT, p.hp / MAX_HP, HP_BUFFER_BODY));
            lines.extend(bar(at, MP_BAR_LIFT, p.mp / MAX_MP, MP_BUFFER_BODY));
        }
        for m in &self.monsters {
            let at = m.body.position;
            lines.extend(bar(at, HP_BAR_LIFT, m.hp / m.max_hp, HP_BUFFER_BODY));
        }
        let mut circles = Vec::new();
        let bite_reach = self.settings.bite_reach;
        let reaches = (self
            .team
            .iter()
            .map(|p| (&p.body, RANGED_REACH, MELEE_REACH)))
        .chain(
            self.monsters
                .iter()
                .map(|m| (&m.body, ALARM_REACH, bite_reach)),
        );
        for (b, big, small) in reaches {
            circles.push(circle_view(b.position, big, color::RANGED));
            circles.push(circle_view(b.position, small, color::MELEE));
        }
        for m in self.monsters.iter().filter(|m| m.target.is_some()) {
            circles.push(circle_view(m.body.position, LOS_REACH, color::SIGHT));
        }
        for p in &self.team {
            if let Some(t) = p.target {
                let at = self.monsters[t].body.position;
                lines.push(line(p.body.position, at, color::ATTACK));
                circles.push(circle_view(at, AOE_BLAST, color::ATTACK));
            }
            if self.time < p.hp_buff_until {
                circles.push(circle_view(p.body.position, HP_BUFF_RING, HP_BUFFER_BODY));
            }
            if self.time < p.mp_buff_until {
                circles.push(circle_view(p.body.position, MP_BUFF_RING, MP_BUFFER_BODY));
            }
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
            circles.push(circle_view(CENTRE, r, color::RADIUS));
            labels.push(label(CENTRE + Vec2::new(0.0, -r - 1.0), text, color::LABEL));
        }
        for m in &self.monsters {
            if let Some(name) = m.rarity.name() {
                let at = m.body.position + Vec2::new(-BAR_WIDTH / 2.0, -HP_BAR_LIFT - 0.6);
                labels.push(label(at, name, color::LABEL));
            }
        }
        let gaits: Vec<String> = (Gait::ALL.iter())
            .map(|g| {
                let n = self.monsters.iter().filter(|m| m.gait == *g).count();
                format!("{n} {}", g.name())
            })
            .collect();
        let chasing = self.monsters.iter().filter(|m| m.target.is_some()).count();
        labels.push(label(
            corner(-w, h + 3.0),
            format!(
                "{} monsters: {}; {chasing} chasing; {} kills, {} deaths",
                self.monsters.len(),
                gaits.join(", "),
                self.kills,
                self.deaths
            ),
            color::LABEL,
        ));
        let lure = &self.lure;
        labels.push(label(
            corner(-w, h + 7.0),
            format!(
                "lurer {:?}: {} trips, {} delivered, {} lost, {:.0}% of the time in the dps disk; {} tags, {} blasts, {} pierces",
                lure.stage,
                lure.trips,
                lure.delivered,
                lure.lost,
                100.0 * lure.share_in_disk(),
                lure.tags,
                lure.blasts,
                lure.pierces
            ),
            color::LABEL,
        ));
        labels.push(label(
            corner(-w, h + 11.0),
            format!(
                "sessions: {} done, {:.1} s each, back with {:.0} HP (lowest {:.0}), {:.0} walked per delivered",
                lure.sessions,
                lure.mean_session(),
                lure.mean_hp_back(),
                if lure.sessions > 0 { lure.hp_back_min } else { 0.0 },
                lure.walked / lure.delivered.max(1) as f32
            ),
            color::LABEL,
        ));
        if lure.stage == LureStage::Out {
            let mut from = self.team[0].body.position;
            for stop in &lure.tour {
                if let Some(m) = self.monsters.iter().find(|m| m.id == stop.id) {
                    lines.push(line(from, m.body.position, color::PATH));
                    from = m.body.position;
                }
            }
            lines.push(line(from, CENTRE, color::PATH));
        }
        let body = |p: &Player| match p.role {
            _ if p.is_dead() => color::BODY,
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
            circles,
            trails: Vec::new(),
            labels,
            legend: legend(&[
                (LURER_BODY, "our lurer"),
                (DPS_BODY, "dps (3)"),
                (
                    HP_BUFFER_BODY,
                    "HP buffer; the HP bar over every body; a ring: its buff running",
                ),
                (
                    MP_BUFFER_BODY,
                    "MP buffer; the MP bar over every player; a ring: its buff running",
                ),
                (MONSTER_BODY, "monster"),
                (
                    color::RADIUS,
                    "training disks: each role's radius round the centre",
                ),
                (color::EDGE, "map edge: monsters spawn inside"),
                (color::TRACK, "the empty part of a bar"),
                (
                    color::RANGED,
                    "big disk: a player's ranged reach 8 (attack, buffs), a monster's alarm 6",
                ),
                (color::MELEE, "melee reach 2: a monster's attack"),
                (
                    color::SIGHT,
                    "line of sight 35: a chasing monster keeps its target inside it",
                ),
                (
                    color::PATH,
                    "the lurer's planned tour: its stops, then home",
                ),
                (color::TARGET, "chase: the player a monster closes on"),
                (color::BODY, "a dead player, back at the centre in 5 s"),
                (
                    color::ATTACK,
                    "attack: a dps's target and its blast (2); the lurer's next tag",
                ),
            ]),
            actions: vec![
                button(
                    "plan_casts",
                    "Plan casts (AoE)",
                    self.settings.planner == Planner::Casts,
                ),
                button(
                    "plan_stops",
                    "Plan stops",
                    self.settings.planner == Planner::Stops,
                ),
            ],
            params: vec![
                slider("monsters", "Monsters", MONSTERS, self.wanted as f32),
                slider(
                    "chasers",
                    "N: back with chasers",
                    CHASERS,
                    self.settings.chasers as f32,
                ),
                slider(
                    "monster_speed",
                    "Monster speed",
                    MONSTER_SPEED,
                    self.settings.monster_speed,
                ),
                slider("bite", "Bite damage", BITE_RANGE, self.settings.bite),
                slider(
                    "bite_reach",
                    "Bite reach",
                    BITE_REACH,
                    self.settings.bite_reach,
                ),
                slider(
                    "load_cap",
                    "Load cap (0: none)",
                    LOAD_CAP,
                    self.settings.load_cap as f32,
                ),
            ],
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
            gaits.push(hunt(&mut body, target, MELEE_REACH, RUN_SPEED, 1.0 / 60.0));
        }
        let gap = (target - body.position).length();
        assert!(gap <= MELEE_REACH && gap > MELEE_REACH - 0.1, "{gap}");
        assert_eq!(body.speed, 0.0);
        assert_eq!(gaits[0], Gait::Running);
        assert_eq!(gaits[179], Gait::Standing);
        // Only players inside the alarm disk count, the nearest first; 7 is inside a player's
        // reach but outside a monster's alarm.
        let players = [
            Vec2::new(0.0, -7.0),
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
                    // Started inside the alarm disk, kept inside the line of sight.
                    assert!(d <= LOS_REACH + 2.0 * DEFAULT_MONSTER_SPEED / 60.0, "{d}");
                    assert!(m.gait != Gait::Walking);
                }
            }
        }
        assert!(chase_steps > 60, "{chase_steps}");
        let view = field.view();
        assert!(view.lines.iter().any(|l| l.color == color::TARGET));
        // 36 entities with two reach circles each, and the three training disks.
        let count = |c: u32| view.circles.iter().filter(|x| x.color == c).count();
        let bodies = 6 + field.monsters().len();
        assert_eq!(
            (count(color::RANGED), count(color::MELEE)),
            (bodies, bodies)
        );
        assert_eq!(count(color::RADIUS), 3);
    }

    #[test]
    fn bars_start_full_and_only_players_regenerate_a_point_a_second() {
        // A quiet field: the lurer's trip finds nothing and nobody bites it.
        let mut field = quiet_field();
        assert!(field
            .team()
            .iter()
            .all(|p| p.hp == MAX_HP && p.mp == MAX_MP));
        assert!(field.monsters().iter().all(|m| m.hp == m.max_hp));
        // Above the buff threshold, so only regeneration moves them. The monster is kept
        // standing 20 from home (at home it would be made whole).
        field.team[0].hp = 92.0;
        field.team[0].mp = 92.0;
        let m = &mut field.monsters[0];
        m.set_rarity(Rarity::Normal);
        m.hp = 50.0;
        m.body = standing(m.spawn + Vec2::new(20.0, 0.0));
        m.gait = Gait::Standing;
        m.until = f32::MAX;
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!(
            (field.team()[0].hp - 97.0).abs() < 0.05,
            "{}",
            field.team()[0].hp
        );
        assert!(
            (field.team()[0].mp - 97.0).abs() < 0.05,
            "{}",
            field.team()[0].mp
        );
        assert_eq!(field.monsters()[0].hp, 50.0);
        // The others stay capped at full.
        assert!(field.team()[1..]
            .iter()
            .all(|p| p.hp == MAX_HP && p.mp == MAX_MP));
    }

    #[test]
    fn bars_are_drawn_over_every_body_and_fill_with_the_points() {
        let mut field = TrainingField::default();
        field.team[0].hp = 50.0;
        let view = field.view();
        let hp_bars: Vec<_> = (view.lines.iter())
            .filter(|l| l.color == HP_BUFFER_BODY)
            .collect();
        let mp_bars = view
            .lines
            .iter()
            .filter(|l| l.color == MP_BUFFER_BODY)
            .count();
        assert_eq!(hp_bars.len(), (6 + DEFAULT_MONSTERS) * BAR_STRANDS);
        assert_eq!(mp_bars, 6 * BAR_STRANDS);
        // Every bar lies on a full-width track drawn before it.
        let tracks: Vec<_> = (view.lines.iter())
            .filter(|l| l.color == color::TRACK)
            .collect();
        assert_eq!(tracks.len(), (6 + DEFAULT_MONSTERS + 6) * BAR_STRANDS);
        assert_eq!(tracks[0].to, [BAR_WIDTH / 2.0, -HP_BAR_LIFT]);
        let first_track = view.lines.iter().position(|l| l.color == color::TRACK);
        let first_fill = view.lines.iter().position(|l| l.color == HP_BUFFER_BODY);
        assert!(first_track < first_fill);
        // The lurer's half-full HP bar is half the bar's width, above its centre; its
        // strands sit one gap apart.
        let lurer = &hp_bars[0];
        assert_eq!(lurer.from, [-BAR_WIDTH / 2.0, -HP_BAR_LIFT]);
        assert_eq!(lurer.to, [0.0, -HP_BAR_LIFT]);
        assert_eq!(hp_bars[1].from[1], -HP_BAR_LIFT - BAR_STRAND_GAP);
    }

    /// Steps the field with the lurer put back standing at `at` after every step, so a bite
    /// test has a target that does not run away.
    fn step_with_parked_lurer(field: &mut TrainingField, at: Vec2, steps: usize) {
        for _ in 0..steps {
            field.update(1.0 / 60.0);
            if !field.team[0].is_dead() {
                field.team[0].body = standing(at);
            }
        }
    }

    #[test]
    fn a_chase_lasts_while_the_player_is_in_sight_and_ends_beyond() {
        let mut field = quiet_field();
        // A monster away from the team, alarmed by the lurer parked 5 from it.
        let den = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(den);
        field.monsters[0].spawn = den;
        // (The lurer is parked after each step, so the alarm fires on the second.)
        step_with_parked_lurer(&mut field, den + Vec2::new(0.0, -5.0), 2);
        assert_eq!(field.monsters()[0].target, Some(0));
        let sight = |field: &TrainingField| {
            (field.view().circles.iter())
                .filter(|c| c.color == color::SIGHT)
                .count()
        };
        assert_eq!(sight(&field), 1);
        // The lurer 25 away: outside the alarm disk, inside the line of sight, still chased.
        step_with_parked_lurer(&mut field, den + Vec2::new(0.0, -25.0), 2);
        assert_eq!(field.monsters()[0].target, Some(0));
        assert_eq!(field.monsters()[0].gait, Gait::Running);
        // 45 away: forgotten, threat included; the monster idles, no sight circle.
        field.monsters[0].threat.insert(0, 5.0);
        step_with_parked_lurer(&mut field, den + Vec2::new(0.0, -45.0), 2);
        assert_eq!(field.monsters()[0].target, None);
        assert!(field.monsters()[0].threat.is_empty());
        assert_eq!(sight(&field), 0);
    }

    #[test]
    fn a_monster_in_melee_reach_bites_three_every_one_and_a_half_seconds() {
        let mut field = TrainingField::default();
        field.param("monsters", 5.0);
        // The lurer parked at a monster's melee reach, far from the rest of the team.
        let at = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(at + Vec2::new(0.0, 1.5));
        field.monsters[0].spawn = at + Vec2::new(0.0, 1.5);
        for m in &mut field.monsters[1..] {
            m.body = standing(Vec2::new(-70.0, -50.0));
            m.spawn = m.body.position;
        }
        for m in &mut field.monsters {
            m.set_rarity(Rarity::Normal);
        }
        let hp_after = |field: &mut TrainingField, seconds: f32| {
            step_with_parked_lurer(field, at, (seconds * 60.0) as usize);
            field.team()[0].hp
        };
        // The first bite lands at once, then every 1.5 s; regeneration is 1 a second.
        assert!((hp_after(&mut field, 1.0) - (MAX_HP - BITE + 1.0).min(MAX_HP)).abs() < 0.1);
        let hp_4s = hp_after(&mut field, 3.0);
        // Bites at 0, 1.5 and 3.0 s: 9 off, 4 regained (capped on the way).
        assert!((91.0..=95.0).contains(&hp_4s), "{hp_4s}");
        assert_eq!(field.monsters()[0].target, Some(0));
        assert_eq!(field.monsters()[0].gait, Gait::Standing);
    }

    #[test]
    fn a_dead_player_is_forgotten_and_returns_to_the_centre_after_five_seconds() {
        let mut field = TrainingField::default();
        field.monsters[0].threat.insert(1, 10.0);
        field.monsters[0].target = Some(1);
        field.team[1].hp = 0.0;
        field.team[1].body = standing(Vec2::new(5.0, 5.0));
        field.update(1.0 / 60.0);
        assert!(field.team()[1].is_dead());
        assert_eq!(field.deaths, 1);
        assert!(field.monsters()[0].threat.is_empty());
        assert_ne!(field.monsters()[0].target, Some(1));
        assert_eq!(field.view().vehicles[1].color, color::BODY);
        for _ in 0..(4.5 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[1].is_dead());
        for _ in 0..60 {
            field.update(1.0 / 60.0);
        }
        let p = &field.team()[1];
        assert!(!p.is_dead());
        assert_eq!((p.hp, p.mp), (MAX_HP, MAX_MP));
        assert!(
            (p.body.position - CENTRE).length() < 1.0,
            "{:?}",
            p.body.position
        );
    }

    #[test]
    fn a_monster_chases_whoever_hurt_it_most_before_anyone_near() {
        let threat = HashMap::from([(0, 5.0), (1, 10.0), (2, 10.0)]);
        assert_eq!(threat_target(&threat, &[true; 6]), Some(1));
        assert_eq!(
            threat_target(&threat, &[true, false, true, true, true, true]),
            Some(2)
        );
        assert_eq!(threat_target(&HashMap::new(), &[true; 6]), None);
        // A monster 30 from the lurer (outside the alarm disk, inside the line of sight),
        // hurt by the lurer, runs at the lurer.
        let mut field = quiet_field();
        let far = Vec2::new(24.0, 18.0);
        field.monsters[0].body = standing(far);
        field.monsters[0].threat.insert(0, 1.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.monsters()[0].target, Some(0));
        assert_eq!(field.monsters()[0].gait, Gait::Running);
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!((field.monsters()[0].body.position - far).length() > 15.0);
    }

    #[test]
    fn a_killed_monster_is_replaced_three_seconds_later() {
        let mut field = TrainingField::default();
        field.monsters[0].hp = 0.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.kills, 1);
        assert_eq!(field.monsters().len(), DEFAULT_MONSTERS - 1);
        for _ in 0..(2.5 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        assert_eq!(field.monsters().len(), DEFAULT_MONSTERS - 1);
        for _ in 0..60 {
            field.update(1.0 / 60.0);
        }
        assert_eq!(field.monsters().len(), DEFAULT_MONSTERS);
    }

    /// Five monsters, the first `near` the centre (inside the dps disk), the rest far away in
    /// a corner, every player at its starting place.
    fn field_with_a_monster_at(near: Vec2) -> TrainingField {
        let mut field = TrainingField::default();
        field.param("monsters", 5.0);
        for m in &mut field.monsters {
            m.body = standing(Vec2::new(-70.0, -50.0));
            m.spawn = m.body.position;
            m.set_rarity(Rarity::Normal);
        }
        field.monsters[0].body = standing(near);
        field.monsters[0].spawn = near;
        field
    }

    #[test]
    fn an_area_attack_hits_at_most_the_five_nearest() {
        // Six monsters in a row below the centre, 0.4 apart: every dps and the lurer aim at
        // the first; both blasts cover all six and hit the five nearest, the sixth untouched.
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        for (k, m) in field.monsters.iter_mut().enumerate().skip(1) {
            m.body = standing(Vec2::new(0.0, -4.0 - 0.4 * k as f32));
        }
        field.monsters.push(field.monsters[4].clone());
        field.monsters[5].body = standing(Vec2::new(0.0, -6.0));
        field.update(1.0 / 60.0);
        let hp: Vec<f32> = field.monsters().iter().map(|m| m.hp).collect();
        assert_eq!(hp, [69.0, 69.0, 69.0, 69.0, 69.0, 100.0]);
        assert_eq!((field.lure.blasts, field.lure.tags), (1, 0));
    }

    #[test]
    fn the_lurer_casts_the_skill_that_tags_the_most() {
        let den = Vec2::new(60.0, 40.0);
        // One monster 7 away: the normal attack.
        let mut field = quiet_field();
        field.monsters[0].body = standing(den + Vec2::new(0.0, -7.0));
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(
            (field.lure.tags, field.lure.blasts, field.lure.pierces),
            (1, 0, 0)
        );
        // Three monsters spread sideways within 3 of one 7 away: the blast, all three tagged.
        let mut field = quiet_field();
        field.monsters[0].body = standing(den + Vec2::new(0.0, -7.0));
        field.monsters[1].body = standing(den + Vec2::new(2.5, -7.0));
        field.monsters[2].body = standing(den + Vec2::new(-2.5, -7.0));
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(
            (field.lure.tags, field.lure.blasts, field.lure.pierces),
            (0, 1, 0)
        );
        assert!(field.monsters()[..3]
            .iter()
            .all(|m| m.threat.contains_key(&0)));
        assert!((field.team()[0].mp - (MAX_MP - BLAST_MP)).abs() < 0.1);
        // Four monsters in a line 3 apart out to 11: the pierce beats the blast's three.
        let mut field = quiet_field();
        for (k, m) in field.monsters.iter_mut().take(4).enumerate() {
            m.body = standing(den + Vec2::new(0.0, -2.0 - 3.0 * k as f32));
        }
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(
            (field.lure.tags, field.lure.blasts, field.lure.pierces),
            (0, 0, 1)
        );
        assert!(field.monsters()[..4]
            .iter()
            .all(|m| m.threat.contains_key(&0)));
        assert!((field.team()[0].mp - (MAX_MP - PIERCE_MP)).abs() < 0.1);
    }

    #[test]
    fn rarities_scale_hp_body_and_bite_and_the_mix_is_drawn_as_set() {
        assert_eq!(Rarity::Champion.hp_multiplier(), 2.0);
        assert_eq!(Rarity::Giant.hp_multiplier(), 20.0);
        assert_eq!(Rarity::PartyNormal.hp_multiplier(), 10.0);
        assert_eq!(Rarity::PartyChampion.hp_multiplier(), 20.0);
        assert_eq!(Rarity::PartyGiant.hp_multiplier(), 200.0);
        // Drawn 10,000 times: the mix within a point of its weights, no party giant.
        let mut rng = fastrand::Rng::with_seed(7);
        let mut counts: HashMap<Rarity, u32> = HashMap::new();
        for _ in 0..10_000 {
            *counts.entry(draw_rarity(&mut rng)).or_insert(0) += 1;
        }
        for &(rarity, weight) in &SPAWN_MIX {
            let share = *counts.get(&rarity).unwrap_or(&0) as f32 / 100.0;
            assert!((share - weight as f32).abs() <= 1.0, "{rarity:?}: {share}%");
        }
        // A giant: 2,000 HP, a body of radius 2, a bite of 9; its bar fills by its own max.
        let mut field = quiet_field();
        let den = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(den + Vec2::new(0.0, 1.5));
        field.monsters[0].spawn = field.monsters[0].body.position;
        field.monsters[0].set_rarity(Rarity::Giant);
        field.monsters[0].hp = 1000.0;
        let view = field.view();
        assert_eq!(view.vehicles[6].radius, 2.0);
        assert!(view.labels.iter().any(|l| l.text == "giant"));
        let bar = (view.lines.iter())
            .filter(|l| l.color == HP_BUFFER_BODY)
            .nth(6 * BAR_STRANDS)
            .unwrap();
        assert_eq!(bar.to[0] - bar.from[0], BAR_WIDTH / 2.0);
        step_with_parked_lurer(&mut field, den, 2);
        assert!((field.team()[0].hp - (MAX_HP - BITE * 3.0)).abs() < 0.1);
    }

    #[test]
    fn a_monster_back_home_with_nobody_after_it_is_whole_again() {
        // Hurt and idle at home: whole at once.
        let mut field = quiet_field();
        field.monsters[0].hp = 50.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.monsters()[0].hp, MAX_HP);
        // Hurt and dragged 20 from home with nobody after it: whole once it has walked back.
        let mut field = quiet_field();
        let m = &mut field.monsters[0];
        m.hp = 50.0;
        m.body = standing(m.spawn + Vec2::new(20.0, 0.0));
        for _ in 0..60 * 60 {
            field.update(1.0 / 60.0);
            if field.monsters()[0].hp == MAX_HP {
                break;
            }
        }
        let m = &field.monsters()[0];
        assert_eq!(m.hp, MAX_HP);
        assert!((m.body.position - m.spawn).length() <= IDLE_RADIUS);
        // Hurt by the lurer and chasing it: not healed.
        let mut field = quiet_field();
        field.monsters[0].hp = 50.0;
        field.monsters[0].threat.insert(0, 1.0);
        field.monsters[0].body = standing(Vec2::new(20.0, 0.0));
        field.monsters[0].spawn = Vec2::new(20.0, 0.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.monsters()[0].hp, 50.0);
    }

    /// Five monsters, all far away in a corner: nobody fights.
    fn quiet_field() -> TrainingField {
        field_with_a_monster_at(Vec2::new(-70.0, -50.0))
    }

    #[test]
    fn the_lurer_leaves_a_quiet_disk_as_soon_as_its_hp_is_full() {
        // Full HP, nothing within the lurer's disk: the trip begins at once and, with
        // nothing to tag, turns back at once.
        let mut field = quiet_field();
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.trips, 1);
        // At 85 HP (under the buff threshold) it waits for the HP buff (+4) and regeneration
        // (+1) to bring it to 90: 1 s.
        let mut field = quiet_field();
        field.team[0].hp = 85.0;
        let mut left_at = None;
        for step in 0..300 {
            field.update(1.0 / 60.0);
            if left_at.is_none() && field.lure.trips > 0 {
                left_at = Some(step);
            }
        }
        assert!(matches!(left_at, Some(55..=65)), "{left_at:?}");
        // With four monsters parked in the dps disk the lurer stays; with the cap off, not.
        let parked = || {
            let mut field = quiet_field();
            for (k, m) in field.monsters.iter_mut().take(4).enumerate() {
                m.body = standing(Vec2::new(7.0, k as f32 * 2.0 - 3.0));
                m.spawn = m.body.position;
            }
            field
        };
        let mut field = parked();
        for _ in 0..180 {
            field.update(1.0 / 60.0);
        }
        assert_eq!((field.lure.stage, field.lure.trips), (LureStage::Centre, 0));
        let mut field = parked();
        field.param("load_cap", 0.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.trips, 1);
    }

    #[test]
    fn the_sliders_set_n_the_monsters_speed_and_the_bite() {
        let mut field = quiet_field();
        field.param("chasers", 5.0);
        field.param("monster_speed", 8.0);
        field.param("bite", 10.0);
        field.param("bite_reach", 4.0);
        assert_eq!(field.settings.chasers, 5);
        assert_eq!(field.settings.monster_speed, 8.0);
        // A monster 3.5 from the parked lurer: inside the new reach, a bite of 10.
        let den = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(den + Vec2::new(0.0, 3.5));
        field.monsters[0].spawn = field.monsters[0].body.position;
        step_with_parked_lurer(&mut field, den, 2);
        assert!(
            (field.team()[0].hp - (MAX_HP - 10.0)).abs() < 0.1,
            "{}",
            field.team()[0].hp
        );
        // A chasing monster runs up to 8 (the integrator's smoothed acceleration takes it a
        // couple of seconds) on its way to the lurer parked 30 away.
        field.monsters[0].threat.insert(0, 1.0);
        let mut top = 0.0f32;
        for _ in 0..240 {
            step_with_parked_lurer(&mut field, den + Vec2::new(0.0, -30.0), 1);
            top = top.max(field.monsters()[0].body.speed);
        }
        assert!(top > 7.9 && top <= 8.0 + 1e-3, "{top}");
        // The params carry the values.
        let params = field.view().params;
        let value = |id: &str| params.iter().find(|p| p.id == id).unwrap().value;
        assert_eq!(
            (value("chasers"), value("bite"), value("load_cap")),
            (5.0, 10.0, 3.0)
        );
    }

    #[test]
    fn the_planner_finds_the_shortest_tour_through_n_stops_and_home() {
        let (a, b, c) = (
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 2.0),
            Vec2::new(-10.0, 0.0),
        );
        let stops = [(1, a), (2, b), (3, c)];
        // Two stops: the pair on the right (22.2) beats any pair with the left one (40+).
        let tour = plan_tour(Vec2::ZERO, Vec2::ZERO, &stops, 2);
        let mut set = tour.clone();
        set.sort();
        assert_eq!(set, [1, 2]);
        // Three: right pair first, then across (42.1), not the left one first (42.2).
        assert_eq!(plan_tour(Vec2::ZERO, Vec2::ZERO, &stops, 3)[2], 3);
        // More asked than there are: all of them; none: empty.
        assert_eq!(plan_tour(Vec2::ZERO, Vec2::ZERO, &stops, 5).len(), 3);
        assert!(plan_tour(Vec2::ZERO, Vec2::ZERO, &[], 3).is_empty());
    }

    #[test]
    fn the_lurer_follows_its_tour_and_records_the_session() {
        // Three monsters in a row to the right, two far off elsewhere: the tour is the row,
        // nearest first, and the lurer sets off along it.
        let mut field = quiet_field();
        let spots = [
            (20.0, 0.0),
            (24.0, 0.0),
            (28.0, 0.0),
            (-40.0, 10.0),
            (0.0, -45.0),
        ];
        for (m, &(x, y)) in field.monsters.iter_mut().zip(&spots) {
            m.body = standing(Vec2::new(x, y));
            m.spawn = m.body.position;
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let tour =
            |field: &TrainingField| -> Vec<u32> { field.lure.tour.iter().map(|s| s.id).collect() };
        // The stops planner; the first step leaves the centre, the second plans.
        field.action("plan_stops");
        field.update(1.0 / 60.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.stage, LureStage::Out);
        assert_eq!(tour(&field), ids[..3]);
        // A second on: heading right along the tour (the low-speed rule bends the first
        // steps toward the starting heading), the tour unchanged by the re-plan.
        for _ in 0..60 {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[0].body.forward.x > 0.9);
        assert_eq!(tour(&field), ids[..3]);
        let tour_lines = (field.view().lines.iter())
            .filter(|l| l.color == color::PATH)
            .count();
        assert_eq!(tour_lines, 4);
        // It tags the row (the pierce takes all three at once, within 12), turns back and
        // is home: one session recorded, with its time and the HP it came back with.
        for _ in 0..60 * 30 {
            field.update(1.0 / 60.0);
            if field.lure.sessions > 0 {
                break;
            }
        }
        let l = &field.lure;
        assert_eq!(l.sessions, 1);
        assert!(
            l.mean_session() > 3.0 && l.mean_session() < 20.0,
            "{}",
            l.mean_session()
        );
        assert!(l.mean_hp_back() > 0.0 && l.hp_back_min == l.mean_hp_back());
        assert!(l.walked > 20.0, "{}", l.walked);
    }

    #[test]
    fn the_cast_planner_tags_a_cluster_at_one_stop_and_a_row_with_one_pierce() {
        // A cluster of three 20 to the right (within one blast) and a lone monster 15 to the
        // left; N = 3. With the blast: one stop at the cluster (12 out, 12 home). Without
        // either skill: three normal attacks, all on the cluster, none on the lone one.
        let mut field = quiet_field();
        let spots = [(20.0, 0.0), (20.0, 2.0), (20.0, -2.0), (-15.0, 0.0)];
        for (m, &(x, y)) in field.monsters.iter_mut().zip(&spots) {
            m.body = standing(Vec2::new(x, y));
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let plan = |field: &TrainingField, skills| {
            plan_casts(
                Vec2::ZERO,
                CENTRE,
                0,
                field.monsters(),
                &ids[..4],
                3,
                skills,
            )
        };
        let with_blast = plan(&field, (true, false));
        assert_eq!(with_blast.len(), 1);
        assert!(ids[..3].contains(&with_blast[0].id));
        assert_eq!(with_blast[0].reach, RANGED_REACH);
        let plain = plan(&field, (false, false));
        assert_eq!(plain.len(), 3);
        assert!(plain.iter().all(|s| ids[..3].contains(&s.id)));
        // Four in a radial row at 20, 23, 26, 29, all four wanted: one pierce aimed at the
        // far end from 12 short of it takes them (34 there and back; with three wanted the
        // planner rightly prefers a normal attack at 20 and a pierce for 23 and 26: 28).
        let mut field = quiet_field();
        for (k, m) in field.monsters.iter_mut().take(4).enumerate() {
            m.body = standing(Vec2::new(20.0 + 3.0 * k as f32, 0.0));
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let pierce = plan_casts(
            Vec2::ZERO,
            CENTRE,
            0,
            field.monsters(),
            &ids[..4],
            4,
            (false, true),
        );
        assert_eq!(
            pierce,
            [Stop {
                id: ids[3],
                reach: PIERCE_LENGTH
            }]
        );
        // The buttons switch the planner.
        assert_eq!(field.settings.planner, Planner::Casts);
        field.action("plan_stops");
        assert_eq!(field.settings.planner, Planner::Stops);
        let active: Vec<_> = (field.view().actions.iter())
            .filter(|a| a.active)
            .map(|a| a.id.clone())
            .collect();
        assert_eq!(active, ["plan_stops"]);
    }

    #[test]
    fn the_lurer_turns_back_with_n_chasers_or_low_hp_or_nothing_left() {
        // Out, an untagged monster 30 away to go for, and N (3) monsters hurt by the lurer in
        // sight: once they chase (after the first step), the next step turns back.
        let out = || {
            let mut field = quiet_field();
            field.lure.stage = LureStage::Out;
            field.monsters[4].body = standing(Vec2::new(30.0, 0.0));
            field
        };
        let mut field = out();
        for m in field.monsters.iter_mut().take(BACK_CHASERS) {
            m.body = standing(Vec2::new(20.0, 0.0));
            m.threat.insert(0, 1.0);
        }
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.stage, LureStage::Out);
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.stage, LureStage::Back);
        // Out at 30% HP: Back at once.
        let mut field = out();
        field.team[0].hp = 30.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.stage, LureStage::Back);
        // Out with every monster beyond the lurer's disk: Back, and home within a few steps.
        let mut field = quiet_field();
        field.lure.stage = LureStage::Out;
        field.team[0].body = standing(Vec2::new(10.0, 0.0));
        field.update(1.0 / 60.0);
        assert_eq!(field.lure.stage, LureStage::Back);
        // Home within a few steps; at full HP it leaves again at once.
        let mut came_home = false;
        for _ in 0..240 {
            field.update(1.0 / 60.0);
            came_home |= field.lure.stage == LureStage::Centre || field.lure.trips >= 1;
        }
        assert!(came_home);
    }

    /// Two minutes of the default field: the lurer makes trips, brings monsters in, and is
    /// out of the dps disk most of the time.
    #[test]
    fn trips_deliver_monsters_to_the_dps_disk() {
        let mut field = TrainingField::default();
        let mut stages = std::collections::HashSet::new();
        for _ in 0..120 * 60 {
            field.update(1.0 / 60.0);
            stages.insert(field.lure.stage);
        }
        let l = &field.lure;
        // Measured 2026-11-02 with normal monsters only: 4 trips, 10 delivered, 0 lost, 12
        // kills, no deaths, the lurer inside the dps disk 32% of the time. With the rarity
        // mix: 4 trips, 12 delivered, 1 lost, 12 kills, 1 death, 50% inside (a death keeps
        // it at the centre 5 s, and a loaded disk keeps it waiting). With monsters at 6 and
        // the leave at full HP: 4 trips, 13 delivered, 1 lost, 10 kills, no deaths, 56%.
        // With the tour planner and the leave at 90%: 5 trips, 9 delivered, 3 lost, 11 kills,
        // 1 death, 47%; 4 sessions of 16.5 s, back with 96 HP (lowest 93), 37 walked per
        // delivered. With the cast planner (the default): 5 trips, 12 delivered, 1 lost, 11
        // kills, 1 death, 39%; 4 sessions of 19.6 s, back with 98 HP (lowest 94), 29 walked
        // per delivered.
        assert_eq!(stages.len(), 3);
        assert!(l.trips >= 3, "{}", l.trips);
        assert!(l.delivered >= 8, "{}", l.delivered);
        assert!(l.lost <= l.delivered / 2, "{} lost", l.lost);
        assert!(l.share_in_disk() < 0.65, "{}", l.share_in_disk());
        assert!(l.sessions >= 3, "{}", l.sessions);
        assert!(
            l.mean_session() > 5.0 && l.mean_session() < 40.0,
            "{}",
            l.mean_session()
        );
        assert!(l.mean_hp_back() > 60.0, "{}", l.mean_hp_back());
        assert!(
            field.kills >= 8 && field.deaths <= 2,
            "{} {}",
            field.kills,
            field.deaths
        );
    }

    #[test]
    fn the_lurer_tags_each_monster_in_reach_once_a_second_and_never_twice() {
        let mut field = quiet_field();
        // Two monsters 7 from the parked lurer on opposite sides (no blast or pierce catches
        // both), away from the team; the first first.
        let den = Vec2::new(60.0, 40.0);
        for (k, m) in field.monsters.iter_mut().take(2).enumerate() {
            m.body = standing(den + Vec2::new(0.0, if k == 0 { 7.0 } else { -7.0 }));
            m.spawn = m.body.position;
        }
        step_with_parked_lurer(&mut field, den, 2);
        let lurer = &field.team()[0];
        assert_eq!(field.monsters()[0].hp, MAX_HP - TAG_DAMAGE);
        assert_eq!(field.monsters()[0].threat, HashMap::from([(0, TAG_DAMAGE)]));
        assert_eq!(field.monsters()[0].target, Some(0));
        assert_eq!(field.monsters()[1].hp, MAX_HP);
        assert!((lurer.mp - (MAX_MP - TAG_MP)).abs() < 0.1);
        // Its target is the monster it tagged this step; next step, the other.
        assert_eq!(lurer.target, Some(0));
        // The second a second later (the cooldown), then nothing left to tag: the first
        // monster, already tagged, is never scratched again.
        step_with_parked_lurer(&mut field, den, 30);
        assert_eq!(field.monsters()[1].hp, MAX_HP);
        step_with_parked_lurer(&mut field, den, 40);
        assert_eq!(field.monsters()[1].hp, MAX_HP - TAG_DAMAGE);
        step_with_parked_lurer(&mut field, den, 180);
        assert_eq!(field.monsters()[0].hp, MAX_HP - TAG_DAMAGE);
        assert_eq!(field.team()[0].target, None);
        // Both tagged monsters stand biting at the parked lurer.
        assert!(field.team()[0].hp < MAX_HP);
    }

    #[test]
    fn the_lurer_holds_its_tag_without_the_mp() {
        let mut field = quiet_field();
        let den = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(den + Vec2::new(0.0, 7.0));
        field.monsters[0].spawn = field.monsters[0].body.position;
        field.team[0].mp = TAG_MP - 1.0;
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(field.monsters()[0].hp, MAX_HP);
        assert_eq!(field.team()[0].target, Some(0));
    }

    #[test]
    fn the_buffers_restore_the_lowest_bar_under_the_threshold_for_ten_seconds() {
        let mut field = quiet_field();
        field.team[1].hp = 50.0;
        field.team[2].mp = 50.0;
        field.update(1.0 / 60.0);
        let t = field.time;
        assert!((field.team()[1].hp_buff_until - (t + BUFF_DURATION)).abs() < 1e-3);
        assert!((field.team()[2].mp_buff_until - (t + BUFF_DURATION)).abs() < 1e-3);
        assert!((field.team()[4].mp - (MAX_MP - HP_BUFF_MP)).abs() < 0.1);
        assert!((field.team()[5].mp - (MAX_MP - MP_BUFF_MP)).abs() < 0.1);
        // 5 s in: 50 + 5 x (4 + 1) HP, 50 + 5 x (3 + 1) MP.
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!(
            (field.team()[1].hp - 75.0).abs() < 0.5,
            "{}",
            field.team()[1].hp
        );
        assert!(
            (field.team()[2].mp - 70.0).abs() < 0.5,
            "{}",
            field.team()[2].mp
        );
        // No second cast while the buff runs: the buffers' MP only regenerated.
        assert!(field.team()[4].mp > MAX_MP - HP_BUFF_MP);
        assert!(field.team()[5].mp > MAX_MP - MP_BUFF_MP);
        // At 10 s the HP buff has given 40 and regeneration 10: full. The MP buff has given
        // 30 and regeneration 10: 90, the threshold, so no fresh cast follows.
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[1].hp >= 99.0, "{}", field.team()[1].hp);
        assert!(
            (field.team()[2].mp - 90.0).abs() < 0.5,
            "{}",
            field.team()[2].mp
        );
    }

    #[test]
    fn a_buff_needs_a_bar_under_the_threshold_within_reach_and_goes_to_the_lowest_first() {
        let mut field = quiet_field();
        // 95%: above the threshold, nothing cast.
        field.team[1].hp = 95.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.team()[1].hp_buff_until, 0.0);
        // The lurer at 50% but 40 away: out of the HP buffer's reach.
        let mut field = quiet_field();
        field.team[0].hp = 50.0;
        field.team[0].body = standing(Vec2::new(40.0, 0.0));
        field.update(1.0 / 60.0);
        assert_eq!(field.team()[0].hp_buff_until, 0.0);
        // Three in need: the lowest first (the buffer heals itself), then the others in order
        // of need, one cast a second, each when in reach of the roaming buffer.
        let mut field = quiet_field();
        field.team[1].hp = 60.0;
        field.team[2].hp = 40.0;
        field.team[4].hp = 30.0;
        let mut buffed_at = [f32::MAX; 6];
        for _ in 0..600 {
            field.update(1.0 / 60.0);
            for (i, p) in field.team().iter().enumerate() {
                if p.hp_buff_until > 0.0 {
                    buffed_at[i] = buffed_at[i].min(field.time);
                }
            }
        }
        assert!(buffed_at[4] < 0.1, "{buffed_at:?}");
        assert!(
            buffed_at[4] < buffed_at[2] && buffed_at[2] < buffed_at[1],
            "{buffed_at:?}"
        );
        assert!(buffed_at[1] < 10.0, "{buffed_at:?}");
        assert!(buffed_at[0] == f32::MAX && buffed_at[3] == f32::MAX && buffed_at[5] == f32::MAX);
    }

    #[test]
    fn the_three_dps_fire_at_a_monster_in_the_disk_and_feed_its_threat() {
        // 4 below the centre: within 8 of all three dps (5.7, 7.7 and 2.1 away).
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        field.update(1.0 / 60.0);
        let m = &field.monsters()[0];
        // Three shots, and the lurer's tag (4 away is inside its reach too).
        assert_eq!(m.hp, MAX_HP - 3.0 * AOE_DAMAGE - TAG_DAMAGE);
        let threat = HashMap::from([(0, 1.0), (1, 10.0), (2, 10.0), (3, 10.0)]);
        assert_eq!(m.threat, threat);
        assert_eq!(m.target, Some(1));
        for (i, p) in field.team().iter().enumerate() {
            let spent = match p.role {
                Role::Dps => AOE_MP,
                Role::Lurer => TAG_MP,
                _ => 0.0,
            };
            assert_eq!(p.target, (spent > 0.0).then_some(0), "player {i}");
            assert!((p.mp - (MAX_MP - spent)).abs() < 0.1, "player {i}");
        }
        // Volleys at 0, 1.5, 3 and 4.5 s: dead at the fourth, replaced 3 s later.
        for _ in 0..(5.0 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        assert_eq!(field.kills, 1);
        assert_eq!(field.monsters().len(), 4);
    }

    #[test]
    fn the_blast_hurts_every_monster_within_two_of_the_target_only() {
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        field.monsters[1].body = standing(Vec2::new(0.0, -5.5));
        field.monsters[2].body = standing(Vec2::new(0.0, -7.5));
        field.update(1.0 / 60.0);
        let hp: Vec<f32> = field.monsters().iter().map(|m| m.hp).collect();
        // The lurer's blast round the middle one (1.5 and 2 away) tags all three.
        assert_eq!(hp[..3], [69.0, 69.0, 99.0]);
    }

    #[test]
    fn a_dps_waits_for_mp_and_runs_into_reach_first() {
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        for p in &mut field.team {
            p.mp = AOE_MP - 1.0;
        }
        field.update(1.0 / 60.0);
        // No shot; only the lurer's tag.
        assert_eq!(field.monsters()[0].hp, MAX_HP - TAG_DAMAGE);
        for _ in 0..70 {
            field.update(1.0 / 60.0);
        }
        assert!(field.monsters()[0].hp <= MAX_HP - AOE_DAMAGE);
        // A monster inside the disk but out of reach of the dps at (4, 0): 10.3 away.
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -9.5));
        field.update(1.0 / 60.0);
        let dps = &field.team()[1];
        assert_eq!(dps.target, Some(0));
        assert!(dps.body.speed > 0.0);
        for _ in 0..120 {
            field.update(1.0 / 60.0);
        }
        let dps = &field.team()[1];
        let gap = (field.monsters()[0].body.position - dps.body.position).length();
        assert!(gap <= RANGED_REACH + 0.5, "{gap}");
        assert!(field.monsters()[0].hp <= MAX_HP - AOE_DAMAGE);
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
    /// dps average 2.5-2.7 in their 10; the buffers 1.5-1.6 in their 5; nobody below 0.76.
    /// Spots cover the disk: each dps spends 25-55% of the time inside the buffers' disk.
    /// (The lurer no longer roams: on a quiet field it waits at the centre.) Spots cover the disk: the dps spend 42-49% of the time inside the
    /// buffers' disk (the user 2026-11-02: with wander they never came in), the lurer 5%.
    #[test]
    fn players_run_inside_their_disks() {
        let mut field = quiet_field();
        let m = measure(&mut field, 60.0);
        assert!(m.player_over <= 0.0, "{}", m.player_over);
        assert!(
            m.mean_speed[1..4].iter().all(|&s| s > 2.3),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.mean_speed[4..].iter().all(|&s| s > 1.4),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.min_speed[1..].iter().all(|&s| s > 0.7),
            "{:?}",
            m.min_speed
        );
        assert!(
            m.in_buff_disk[1..4].iter().all(|&s| s > 0.2),
            "{:?}",
            m.in_buff_disk
        );
        assert!(
            m.in_buff_disk[4..].iter().all(|&s| s > 0.99),
            "{:?}",
            m.in_buff_disk
        );
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
                let top = speed_of(m.gait, field.settings.monster_speed);
                assert!(
                    m.body.speed <= top + 1e-3,
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
