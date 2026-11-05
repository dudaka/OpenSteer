//! "Training Field": a Silkroad training spot in miniature, built to work on the lurer. A team
//! of eight trains round one centre, each role keeping to its own radius, in Silkroad's world
//! units since the watch log (2026-11-05): four dps and the MP buffer (20), the HP buffer (5)
//! and two lurers (40); monsters spawn at nests on a disk map (70).
//! Built so far (PLAN.md, Phase 8): the map, the disks, the spawns, and bodies at human speeds,
//! each roaming its own disk from one random spot to the next: the players always run, a
//! monster stands, walks or runs for a few seconds at a time. Every entity carries two disks
//! on its own position: a big one, its ranged reach (a player's 15 is its attack and its buffs,
//! a monster's 8 is its alarm), and a small one, its melee reach (4). Players are ranged; a
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

use std::collections::{HashMap, VecDeque};
use std::f32::consts::TAU;

use steer_core::containment::{avoid, contain};
use steer_core::obstacle::{avoid_obstacles, Circle, SeenFrom};
use steer_core::wall::Wall;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle_view, color, label, legend, line, vehicle_view};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

/// Units: 1 = 1 world unit of Silkroad (10 wire; a "space" in PhBot's settings), as the
/// Caravan watch log measured a real party (2026-11-05, `PLAN.md` Findings): the dps stayed
/// within 17.5 of the centre at p90, the HP buffer within 4.5, the MP buffer roamed like a
/// dps, the lurers' trips reached 36-38 at p50 and 45 at p90 (PhBot's "walk 40"), monsters
/// were first seen 40 away at p50 and 68 at p90.
/// The map: a disk of this radius round the training centre (the user: a rectangle left
/// monsters stuck in its corners). Nests lie inside it by an idle radius.
pub const MAP_RADIUS: f32 = 70.0;
pub const CENTRE: Vec2 = Vec2::ZERO;
pub const DPS_RADIUS: f32 = 20.0;
pub const BUFF_RADIUS: f32 = 5.0;
pub const MP_BUFF_RADIUS: f32 = 20.0;
pub const LURE_RADIUS: f32 = 40.0;
/// How far from its spawn an idle monster wanders (the log: its legs ended 12 from where it
/// was first seen at p50), and how long it keeps a gait.
pub const IDLE_RADIUS: f32 = 12.0;
const GAIT_MIN_S: f32 = 2.0;
const GAIT_MAX_S: f32 = 6.0;
const MONSTERS: Range = Range::new(5.0, 60.0, 1.0);
const DEFAULT_MONSTERS: usize = 15;
const SEED: u64 = 1;
/// A body seen from above, 2 units across: a person at the map's scale (OpenSteer's default
/// 0.5 reads as a dot on a 160-unit map).
pub const BODY_RADIUS: f32 = 1.0;
/// The walk is a monster's walking gait (the Sand worm's 4); the run is a player's, as the
/// party ran all the time with its speed buffs (10.2-12.2; gamedata's bare 5 was never seen).
/// Players always run.
pub const WALK_SPEED: f32 = 4.0;
pub const RUN_SPEED: f32 = 11.0;
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
pub const AOE_BLAST: f32 = 5.0;
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
/// The HP buffer's further skills (the user: fill the gap a giant opens), each tried before
/// the HP buff, still one cast a second: a heal on a member under `HEAL_AT` of its HP, a
/// group heal when `GROUP_HEAL_MEMBERS` or more members in reach are under `GROUP_HEAL_AT`
/// percent (bar by bar; on the party's mean it never fired), a guard on the member under
/// attack with the lowest HP (bites on it times `GUARD_FACTOR`).
pub const HEAL_AT: f32 = 0.5;
pub const HEAL_HP: f32 = 40.0;
pub const HEAL_MP: f32 = 15.0;
pub const HEAL_COOLDOWN: f32 = 3.0;
pub const GROUP_HEAL_AT: f32 = 70.0;
pub const GROUP_HEAL_MEMBERS: usize = 2;
pub const GROUP_HEAL_HP: f32 = 20.0;
pub const GROUP_HEAL_MP: f32 = 25.0;
pub const GROUP_HEAL_COOLDOWN: f32 = 10.0;
pub const GUARD_FACTOR: f32 = 0.8;
pub const GUARD_DURATION: f32 = 20.0;
pub const GUARD_MP: f32 = 10.0;
pub const GUARD_COOLDOWN: f32 = 5.0;
const GUARD_RING: f32 = 1.7;
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
/// The game shows a player the entities within about 30 (the user): the lurer knows only the
/// monsters within `LURER_SIGHT` of it, and with none in sight scouts spots drawn in the band
/// between `LURER_SIGHT` and the luring disk's edge (less a margin), where the rest hide. It
/// may tag a monster beyond the luring disk when a skill reaches it from inside (the disk
/// fences the lurer's walk, not its targets).
pub const LURER_SIGHT: f32 = 30.0;
pub const SCOUT_MARGIN: f32 = 3.0;
/// The tour planner: the N stops (untagged monsters in the lurer's disk) and their order that
/// make the shortest run from the lurer through them and home, solved exactly over the
/// `TOUR_CANDIDATES` nearest and planned again every `PLAN_PERIOD`.
pub const TOUR_CANDIDATES: usize = 8;
pub const PLAN_PERIOD: f32 = 1.0;
/// The team: the first `LURERS` players are the lurers (A and B), then the dps, then the
/// buffers. A lurer after another waits for it to be `WATCH_DISTANCE` out before leaving,
/// at most `WATCH_TIMEOUT` (the user: watch the other before starting; no deadlock, since
/// the first never waits and the wait has a timeout).
pub const LURERS: usize = 2;
pub const WATCH_DISTANCE: f32 = 15.0;
pub const WATCH_TIMEOUT: f32 = 5.0;
/// How long a lurer remembers where it last saw the other.
pub const SEEN_MEMORY: f32 = 10.0;
/// The lurer's risk check (the user: read the dps disk and the party before tagging a strong
/// monster), from the party window bar by bar (the party's mean hid a dps at 11% behind
/// seven near 100%): a strong monster is a candidate only while every member's HP is at
/// least `RISKY_HP` percent and none fell more than `RISKY_DROP` points over `TREND_WINDOW`;
/// with a member under `HALT_HP` or fallen more than `HALT_DROP`, no session starts.
pub const RISKY_HP: f32 = 70.0;
pub const RISKY_DROP: f32 = 15.0;
pub const HALT_HP: f32 = 50.0;
pub const HALT_DROP: f32 = 30.0;
/// A strong monster is a candidate only while the HP buffer's MP bar (the party window shows
/// MP too) is at least this: at every death seen the healer had 5-13 MP with 2-4 strong
/// monsters in the disk; its MP, not the dps's damage, is the party's sustain.
pub const HEALER_MP: f32 = 50.0;
pub const TREND_WINDOW: f32 = 5.0;
/// The balance sliders and their defaults: N, the monsters' running speed (the Sand worm's
/// 10, a little under the buffed party's 11), the bite's damage and reach, the backlog cap: a lurer waits
/// while the dps work it knows of (the HP it knows of in the dps disk, each monster's max
/// HP less the damage the lurer saw land on it, at the four dps's focused output `DPS_RATE`,
/// in seconds, read with the disk in sight and run down since, plus what chases the lurer
/// itself) is above this many seconds, and takes a strong monster only
/// under it (the user, 2026-11-03: work left, not head counts; a giant is 20 normals). Off
/// by default: at 15 and 30 monsters the lowest-bar risk check alone kept everyone alive
/// while every cap only cost kills (the tables in `PLAN.md` Findings).
const CHASERS: Range = Range::new(1.0, 10.0, 1.0);
const MONSTER_SPEED: Range = Range::new(5.0, 15.0, 0.5);
const BITE_RANGE: Range = Range::new(1.0, 10.0, 0.5);
const BITE_REACH: Range = Range::new(2.0, 8.0, 0.5);
const BACKLOG_CAP: Range = Range::new(0.0, 120.0, 5.0);
pub const DEFAULT_BACKLOG_CAP: f32 = 0.0;
pub const DPS_RATE: f32 = 4.0 * AOE_DAMAGE / AOE_COOLDOWN;
pub const BACK_CHASERS: usize = 3;
pub const DEFAULT_MONSTER_SPEED: f32 = 10.0;
/// Rocks: random circle obstacles inside the luring disk (the user, 2026-11-03), drawn from
/// their own rng after the nests so the monsters' draws stay as without them; radii
/// `ROCK_MIN..ROCK_MAX`, clear of the dps disk by `ROCK_DISK_GAP`, of a nest's idle disk by a
/// body, of each other by `ROCK_GAP`. Runners outside the dps disk steer round them
/// `AVOID_TIME` ahead (OpenSteer's obstacle avoidance, which wins over the seek).
const OBSTACLES: Range = Range::new(0.0, 20.0, 1.0);
pub const DEFAULT_OBSTACLES: usize = 8;
const ROCK_SEED: u64 = 2;
pub const ROCK_MIN: f32 = 1.5;
pub const ROCK_MAX: f32 = 5.0;
pub const ROCK_GAP: f32 = 3.0;
pub const ROCK_DISK_GAP: f32 = 1.0;
const ROCK_TRIES: usize = 1000;
pub const AVOID_TIME: f32 = 2.0;

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
    /// The lurer stays while the backlog it knows of is above this many seconds; 0 for no
    /// cap.
    pub backlog_cap: f32,
    /// How many rocks the luring disk holds.
    pub obstacles: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            planner: Planner::Casts,
            chasers: BACK_CHASERS,
            monster_speed: DEFAULT_MONSTER_SPEED,
            bite: BITE,
            bite_reach: MELEE_REACH,
            backlog_cap: DEFAULT_BACKLOG_CAP,
            obstacles: DEFAULT_OBSTACLES,
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
/// buffs: the party's skills have range 150, 15 here; a monster's alarm, smaller: a monster's
/// first unprovoked leg at a player began 7.6 away at p50) and the small disk (melee; a
/// monster's attack: bites landed 4.4 away at p50).
pub const RANGED_REACH: f32 = 15.0;
pub const ALARM_REACH: f32 = 8.0;
pub const MELEE_REACH: f32 = 4.0;
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
            Role::HpBuffer => BUFF_RADIUS,
            Role::MpBuffer => MP_BUFF_RADIUS,
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
    /// The HP buffer's skill cooldowns.
    heal_ready: f32,
    group_ready: f32,
    guard_ready: f32,
    /// Guarded (bites do less) until this time.
    pub guard_until: f32,
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

    /// Strong: HP ten times a normal's or more (the giants and every party monster).
    pub fn is_strong(self) -> bool {
        self.hp_multiplier() >= 10.0
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
    was_chasing_lurer: Option<usize>,
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
    /// over by someone's damage), except a packmate a dps splashed at the disk's edge, which
    /// counts delivered.
    pub lost: u32,
    /// Steps the lurer spent inside the dps disk, of all steps.
    pub steps_in_disk: u32,
    pub steps: u32,
    /// Casts: normal attacks, blasts and pierces; and the count when the trip left (a trip
    /// that has tagged something and sees nothing more turns home with what follows).
    pub tags: u32,
    pub blasts: u32,
    pub pierces: u32,
    left_casts: u32,
    /// With nothing in sight, the spot in the band the lurer scouts toward.
    pub scout: Option<Vec2>,
    /// Since when it has been ready to leave but watching an earlier lurer.
    ready_since: Option<f32>,
    /// Where and when it last saw each other lurer (memory, `SEEN_MEMORY` long).
    last_seen: Vec<Option<(Vec2, f32)>>,
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
    /// The backlog last read with the dps disk in sight: seconds of dps work, and when.
    backlog_seen: Option<(f32, f32)>,
    /// The damage this lurer has seen land on each monster (hits within its sight), by id:
    /// the server sends no HP for a monster a player has not selected, so a monster's HP is
    /// known only as its max HP less this tally (a killed monster's id never returns; a
    /// monster made whole at home is still counted hurt, as a player would).
    damage_seen: HashMap<u32, f32>,
    /// The backlog this lurer knew of after the last step.
    pub known_backlog: f32,
}

impl Lure {
    pub fn new() -> Lure {
        Lure {
            last_seen: vec![None; LURERS],
            ..Lure::default()
        }
    }

    /// The dps work the lurer knows of, in seconds: what it last saw in the disk less the
    /// time since (the dps kept firing), plus `chasing_hp`, the HP of what chases it from
    /// outside the disk.
    pub fn backlog(&self, now: f32, chasing_hp: f32) -> f32 {
        let seen = (self.backlog_seen).map_or(0.0, |(s, t)| (s - (now - t)).max(0.0));
        seen + chasing_hp / DPS_RATE
    }

    /// A monster's HP as this lurer knows it: its max HP less the damage it saw hit it.
    pub fn known_hp(&self, m: &Monster) -> f32 {
        (m.max_hp - self.damage_seen.get(&m.id).copied().unwrap_or(0.0)).max(0.0)
    }

    fn see_hit(&mut self, id: u32, damage: f32) {
        *self.damage_seen.entry(id).or_insert(0.0) += damage;
    }

    pub fn share_in_disk(&self) -> f32 {
        self.steps_in_disk as f32 / self.steps.max(1) as f32
    }

    pub fn casts(&self) -> u32 {
        self.tags + self.blasts + self.pierces
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
    /// The rocks in the luring disk.
    obstacles: Rocks,
    /// How many monsters the map keeps alive (the slider).
    wanted: usize,
    /// Killed monsters' nests and when each respawns there (the game's nests are fixed: a
    /// field whose respawns landed anywhere drained the lurer's disk).
    respawns: Vec<(f32, Vec2)>,
    pub kills: u32,
    /// The HP buffer's casts.
    pub heals: u32,
    pub group_heals: u32,
    pub guards: u32,
    next_id: u32,
    pub deaths: u32,
    /// One per lurer (A, B).
    pub lures: Vec<Lure>,
    /// The party window each step over the last `TREND_WINDOW`: every member's HP in
    /// percent, the dead at 0.
    bars: VecDeque<Vec<f32>>,
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

/// Whether a monster stands inside the dps disk.
fn in_dps_disk(m: &&Monster) -> bool {
    (m.body.position - CENTRE).length() <= DPS_RADIUS
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

/// The rocks in the luring disk: circles and rectangles at any angle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rocks {
    pub circles: Vec<Circle>,
    pub rectangles: Vec<Wall>,
}

impl Rocks {
    pub fn len(&self) -> usize {
        self.circles.len() + self.rectangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The push that keeps `body` off the rocks `AVOID_TIME` ahead: OpenSteer's avoidance
    /// for the circles, else the probe `avoid` for the rectangles widened by the body's
    /// radius; zero when the way is clear.
    fn push(&self, body: &Vehicle) -> Vec2 {
        let push = avoid_obstacles(body, AVOID_TIME, &self.circles);
        if push != Vec2::ZERO {
            return push;
        }
        (self.rectangles.iter())
            .map(|w| avoid(body, &Wall::new(&w.offset(body.radius)), AVOID_TIME))
            .find(|p| *p != Vec2::ZERO)
            .unwrap_or(Vec2::ZERO)
    }

    /// A nest's idle disk and a body's width clear of every rock.
    fn nest_room(&self, nest: Vec2) -> bool {
        let room = IDLE_RADIUS + BODY_RADIUS;
        (self.circles.iter()).all(|o| (nest - o.center).length() >= o.radius + room)
            && (self.rectangles.iter()).all(|w| !w.contains(nest) && w.distance(nest) >= room)
    }
}

/// A rectangle with half sides `a` and `b`, turned by `angle`, as a counter-clockwise wall.
fn rectangle(centre: Vec2, a: f32, b: f32, angle: f32) -> Wall {
    let (sin, cos) = angle.sin_cos();
    let corner = |x: f32, y: f32| centre + Vec2::new(x * cos - y * sin, x * sin + y * cos);
    Wall::new(&[corner(a, b), corner(-a, b), corner(-a, -b), corner(a, -b)])
}

/// `count` rocks from their own rng, each a circle or a rectangle (a coin): a circle's radius
/// and a rectangle's half sides within `ROCK_MIN..ROCK_MAX`, the rectangle turned by a random
/// angle; inside the luring disk, clear of the dps disk by `ROCK_DISK_GAP`, of every nest's
/// idle disk by a body and of each other by `ROCK_GAP`, all by the rock's bounding radius; a
/// rock that finds no room in `ROCK_TRIES` draws ends the list. Drawn one after another, so
/// the first k rocks of a longer list are the k of a shorter one.
fn place_rocks(count: usize, nests: &[Vec2]) -> Rocks {
    let mut rng = fastrand::Rng::with_seed(ROCK_SEED);
    let mut rocks = Rocks::default();
    // Each placed rock's centre and bounding radius.
    let mut taken: Vec<(Vec2, f32)> = Vec::new();
    for _ in 0..count {
        let found = (0..ROCK_TRIES).find_map(|_| {
            let round = rng.bool();
            let mut side = || ROCK_MIN + (ROCK_MAX - ROCK_MIN) * rng.f32();
            let (a, b) = (side(), side());
            let bound = if round { a } else { a.hypot(b) };
            let at = random_spot(CENTRE, LURE_RADIUS - bound, &mut rng);
            let angle = rng.f32() * TAU / 2.0;
            let clear = (at - CENTRE).length() - bound >= DPS_RADIUS + ROCK_DISK_GAP
                && (nests.iter()).all(|&n| (n - at).length() >= bound + IDLE_RADIUS + BODY_RADIUS)
                && (taken.iter()).all(|&(c, r)| (at - c).length() >= bound + r + ROCK_GAP);
            clear.then_some((round, at, a, b, angle, bound))
        });
        let Some((round, at, a, b, angle, bound)) = found else {
            break;
        };
        taken.push((at, bound));
        if round {
            rocks.circles.push(Circle::new(at, a, SeenFrom::Outside));
        } else {
            rocks.rectangles.push(rectangle(at, a, b, angle));
        }
    }
    rocks
}

/// The disk a body roams and the rocks it may meet on its way back into it.
#[derive(Clone, Copy)]
struct Patch<'a> {
    centre: Vec2,
    radius: f32,
    rocks: &'a Rocks,
}

/// One step of `body` at `gait` roaming the disk (`centre`, `radius`) with its whole body
/// inside: standing stops it; otherwise it seeks `spot`, drawing a new random spot of the disk
/// each time it comes within `SPOT_REACHED` (the game bots' random walk, which covers the whole
/// disk; a wanderer held by containment only circles along the edge). Containment stays as the
/// guard while the probe ahead is outside, a runner braking into it. Outside the disk (a
/// monster walking home from a chase) the spot leads home, no guard, and `rocks` are avoided.
fn roam(
    body: &mut Vehicle,
    gait: Gait,
    run_speed: f32,
    spot: &mut Vec2,
    patch: Patch,
    dt: f32,
    rng: &mut fastrand::Rng,
) {
    let Patch {
        centre,
        radius,
        rocks,
    } = patch;
    let room = radius - BODY_RADIUS;
    body.max_speed = speed_of(gait, run_speed);
    if (*spot - body.position).length() < SPOT_REACHED {
        *spot = random_spot(centre, room, rng);
    }
    let inside = (body.position - centre).length() <= room;
    let push = if inside {
        contain(body, &disk(centre, room), LOOK_AHEAD)
    } else {
        Vec2::ZERO
    };
    let avoid = if inside { Vec2::ZERO } else { rocks.push(body) };
    let force = if gait == Gait::Standing {
        Vec2::ZERO
    } else if push != Vec2::ZERO {
        if gait == Gait::Running {
            body.apply_braking_force(CONTAIN_BRAKE, dt);
        }
        push
    } else if avoid != Vec2::ZERO {
        avoid
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
/// steering round `rocks` first, a stop (facing it) once inside.
fn hunt(
    body: &mut Vehicle,
    target: Vec2,
    reach: f32,
    run_speed: f32,
    dt: f32,
    rocks: &Rocks,
) -> Gait {
    let gait = if (target - body.position).length() > reach {
        Gait::Running
    } else {
        Gait::Standing
    };
    body.max_speed = speed_of(gait, run_speed);
    let force = match gait {
        Gait::Running => {
            let avoid = rocks.push(body);
            if avoid != Vec2::ZERO {
                avoid
            } else {
                steer::seek(body, target)
            }
        }
        _ => Vec2::ZERO,
    };
    body.apply_steering_force(force, dt);
    gait
}

/// A bar as the party window shows it: the game's party packet carries each member's HP and
/// MP as tenths, so a bar reads 70 from 70% up to 80% (the floor of HP over a tenth of the
/// maximum, never a fraction of the percent).
pub fn window_tenth(value: f32, max: f32) -> f32 {
    (value / (max / 10.0)).floor().clamp(0.0, 10.0) * 10.0
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
/// Whether any lurer has tagged `m` (its threat table names one).
fn tagged_by_a_lurer(m: &Monster) -> bool {
    (0..LURERS).any(|l| m.threat.contains_key(&l))
}

/// A monster no lurer pulls: tagged by a lurer, already chasing a player, or already inside
/// the dps disk (the game shows all three: the lurer's own hits, whom a monster chases, where
/// it stands). Without the last two, lurers freed by a high cap stepped out, tagged a monster
/// fighting the dps at the disk's edge, and stepped back, 75 "trips" of 2 s in three minutes.
fn engaged(m: &Monster) -> bool {
    tagged_by_a_lurer(m) || m.target.is_some() || in_dps_disk(&m)
}

/// Which lurer an untagged monster at `at` belongs to: the nearest living one (`lurers` by
/// index, `None` for a dead one), the lower index on a tie. Both lurers see each other, so
/// this splits the field between them without a word (the user: a lurer must not kite the
/// other's monsters, and cannot know the other's plan).
fn owner(at: Vec2, lurers: &[Option<Vec2>]) -> Option<usize> {
    (lurers.iter().enumerate())
        .filter_map(|(l, p)| p.map(|p| (l, (p - at).length())))
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(l, _)| l)
}

/// The monster nearest to `at` within the lurer's ranged reach that no lurer has tagged and
/// that is this lurer's: the lurer's next normal attack.
fn untagged_in_reach(
    lurer: usize,
    at: Vec2,
    monsters: &[Monster],
    lurers: &[Option<Vec2>],
    strong_ok: bool,
) -> Option<usize> {
    (monsters.iter().enumerate())
        .filter(|(_, m)| !engaged(m) && owner(m.body.position, lurers) == Some(lurer))
        .filter(|(_, m)| strong_ok || !m.rarity.is_strong())
        .map(|(i, m)| (i, (m.body.position - at).length()))
        .filter(|&(_, d)| d <= RANGED_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The cheapest run that tags at least `n` monsters by casts, from `from` through its stops
/// and home: an exact search over `candidates` (monster ids). At each stop the lurer gets
/// within a skill's reach of a monster and casts: the normal attack tags it alone (reach 15),
/// the blast every untagged monster within `LURE_BLAST` of it (reach 15), the pierce every
/// untagged monster on the line from the cast point to it (reach `PIERCE_LENGTH`); the blast
/// and pierce only when `skills` (blast ready, pierce ready) allow, every cast point within
/// `fence` of `home` (the lurer's walk stays in its disk; a target may lie beyond it, the
/// user's rule), and with `avoid_strong` no area cast that would catch a strong monster. The search ends a run
/// once `n` are tagged or nothing more can be, and keeps the shortest, the fewest casts on a
/// tie.
pub fn plan_casts(
    (from, home): (Vec2, Vec2),
    monsters: &[Monster],
    candidates: &[u32],
    n: usize,
    skills: (bool, bool),
    (fence, avoid_strong): (f32, bool),
) -> Vec<Stop> {
    struct Search<'a> {
        home: Vec2,
        fence: f32,
        avoid_strong: bool,
        monsters: &'a [Monster],
        candidates: &'a [u32],
        n: usize,
        skills: (bool, bool),
        best: (f32, Vec<Stop>),
    }
    impl Search<'_> {
        fn untagged(&self, hits: Vec<usize>, tagged: &[u32]) -> Vec<u32> {
            (hits.into_iter().map(|i| &self.monsters[i]))
                .filter(|m| !engaged(m) && !tagged.contains(&m.id))
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
                    let allowed = |hits: &[usize]| {
                        !self.avoid_strong
                            || hits.iter().all(|&i| !self.monsters[i].rarity.is_strong())
                    };
                    if self.skills.0 {
                        let hits = in_blast(target, LURE_BLAST, self.monsters);
                        if allowed(&hits) {
                            let tags = self.untagged(hits, tagged);
                            if tags.len() >= 2 {
                                options.push((RANGED_REACH, tags));
                            }
                        }
                    }
                    if self.skills.1 {
                        let hits = along_pierce(cast_at(PIERCE_LENGTH), target, self.monsters);
                        if allowed(&hits) {
                            let tags = self.untagged(hits, tagged);
                            if tags.len() >= 2 {
                                options.push((PIERCE_LENGTH, tags));
                            }
                        }
                    }
                    for (reach, tags) in options {
                        if tags.is_empty() || (cast_at(reach) - self.home).length() > self.fence {
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
        fence,
        avoid_strong,
        monsters,
        candidates,
        n,
        skills,
        best: (f32::MAX, Vec::new()),
    };
    search.go(from, &mut Vec::new(), 0.0, &mut Vec::new());
    search.best.1
}

/// The planner's candidates: the monsters the lurer sees (within `LURER_SIGHT` of `at`) that
/// no lurer has tagged, that are this lurer's by `owner`, and that a skill could reach from
/// inside its disk, the `TOUR_CANDIDATES` nearest, as (id, position).
fn tour_candidates(
    lurer: usize,
    at: Vec2,
    monsters: &[Monster],
    lurers: &[Option<Vec2>],
    strong_ok: bool,
) -> Vec<(u32, Vec2)> {
    let mut near: Vec<&Monster> = (monsters.iter())
        .filter(|m| !engaged(m) && owner(m.body.position, lurers) == Some(lurer))
        .filter(|m| strong_ok || !m.rarity.is_strong())
        .filter(|m| (m.body.position - at).length() <= LURER_SIGHT)
        .filter(|m| (m.body.position - CENTRE).length() <= LURE_RADIUS + PIERCE_LENGTH)
        .collect();
    near.sort_by(|a, b| {
        let d = |m: &Monster| (m.body.position - at).length();
        d(a).total_cmp(&d(b))
    });
    near.truncate(TOUR_CANDIDATES);
    near.into_iter().map(|m| (m.id, m.body.position)).collect()
}

/// A scouting spot: a random point of the band between the lurer's sight and the luring
/// disk's edge less `SCOUT_MARGIN`, uniform over its area; with another lurer out, on the
/// half of the band away from it.
fn scout_spot(rng: &mut fastrand::Rng, away_from: Option<Vec2>) -> Vec2 {
    let (inner, outer) = (LURER_SIGHT, LURE_RADIUS - SCOUT_MARGIN);
    let r = (inner * inner + (outer * outer - inner * inner) * rng.f32()).sqrt();
    let angle = match away_from {
        // The half of the band facing away from the other lurer.
        Some(other) => {
            let off = other - CENTRE;
            off.y.atan2(off.x) + TAU / 2.0 + (rng.f32() - 0.5) * TAU / 2.0
        }
        None => rng.f32() * TAU,
    };
    around(CENTRE, r, angle)
}

/// Each lurer at the start of a step, before anyone's point of view is applied.
#[derive(Debug, Clone, Copy)]
struct Snap {
    at: Vec2,
    alive: bool,
    /// HP back to `LEAVE_HP`.
    hp_ok: bool,
    speed: f32,
}

/// What one lurer can know of another (itself included), as the game allows: its position and
/// movement only while within the nearby radius (`LURER_SIGHT`), its HP from the party window
/// at any distance. Its plan and its stage it never knows; being out of sight means it is at
/// least 30 away. (A dead lurer is unseen.)
#[derive(Debug, Clone, Copy)]
struct Observed {
    seen: Option<Vec2>,
    hp_ok: bool,
    moving: bool,
}

/// Lurer `me`'s view of every lurer.
fn observe(me: usize, snap: &[Snap]) -> Vec<Observed> {
    (snap.iter().enumerate())
        .map(|(j, l)| {
            let near = j == me || (l.at - snap[me].at).length() <= LURER_SIGHT;
            Observed {
                seen: (l.alive && near).then_some(l.at),
                hp_ok: l.alive && l.hp_ok,
                moving: l.alive && l.speed > 0.05,
            }
        })
        .collect()
}

/// What a trip step needs to know.
struct Trip<'a> {
    settings: &'a Settings,
    monsters: &'a [Monster],
    lurers: &'a [Observed],
    /// Monsters chasing this lurer; the dps work it knows of, in seconds.
    chasers: usize,
    backlog: f32,
    /// The risk check: strong monsters may be tagged; no session should start.
    strong_ok: bool,
    halt: bool,
    now: f32,
    dt: f32,
}

/// One step of lurer `me`'s trip. At the centre it stands until nothing chases it any more
/// (the hand-off: the dps have taken its pack; a lurer that left with chasers dragged them
/// back out, 54 of 68 leaves at seed 1), its HP is back to `LEAVE_HP`,
/// the backlog it knows of is within the cap (`Lure::backlog`), the party is not halted,
/// and no earlier lurer it can see is about to leave or just leaving (within
/// `WATCH_DISTANCE`, healthy or moving), a wait of at most `WATCH_TIMEOUT`; out with nothing
/// in sight it scouts the band away from where it last saw the other lurer, having tagged
/// something it turns home with what follows; out, it follows its tour (planned
/// again every `PLAN_PERIOD`, stops already tagged dropped; the tag itself fires from
/// `lure_cast`) until N chase, its HP is under `BACK_HP` or no stop is left; back, it runs
/// home, and on arrival the session's time and HP are recorded.
fn lure_step(
    p: &mut Player,
    me: usize,
    lure: &mut Lure,
    trip: &Trip,
    rng: &mut fastrand::Rng,
    rocks: &Rocks,
) {
    let Trip {
        settings,
        monsters,
        lurers,
        chasers,
        backlog,
        strong_ok,
        halt,
        now,
        dt,
    } = *trip;
    let before = p.body.position;
    let positions: Vec<Option<Vec2>> = lurers.iter().map(|o| o.seen).collect();
    for (j, o) in lurers.iter().enumerate() {
        if let (true, Some(at)) = (j != me, o.seen) {
            lure.last_seen[j] = Some((at, now));
        }
    }
    match lure.stage {
        LureStage::Centre => {
            hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt, rocks);
            let clear = settings.backlog_cap == 0.0 || backlog <= settings.backlog_cap;
            let handed_off = chasers == 0;
            let ready = handed_off && p.hp >= LEAVE_HP * MAX_HP && clear && !halt;
            if !ready {
                lure.ready_since = None;
                return;
            }
            let since = *lure.ready_since.get_or_insert(now);
            // An earlier lurer in sight near the centre, healthy enough to leave or on the
            // move, is about to go or just going: let it get its distance first.
            let blocker = (lurers[..me].iter()).any(|o| {
                o.seen
                    .is_some_and(|at| (at - CENTRE).length() < WATCH_DISTANCE)
                    && (o.hp_ok || o.moving)
            });
            if blocker && now - since < WATCH_TIMEOUT {
                return;
            }
            lure.stage = LureStage::Out;
            lure.trips += 1;
            lure.left_at = now;
            lure.left_casts = lure.casts();
            lure.planned_at = f32::MIN;
            lure.scout = None;
            lure.ready_since = None;
        }
        LureStage::Out => {
            let back = chasers >= settings.chasers || p.hp < BACK_HP * MAX_HP;
            let at = p.body.position;
            let find = |id: u32| monsters.iter().find(|m| m.id == id);
            lure.tour
                .retain(|s| find(s.id).is_some_and(|m| !tagged_by_a_lurer(m)));
            if !back && (lure.tour.is_empty() || now - lure.planned_at >= PLAN_PERIOD) {
                let mut candidates = tour_candidates(me, at, monsters, &positions, strong_ok);
                let n = settings.chasers;
                lure.tour = match settings.planner {
                    Planner::Stops => {
                        // Stops are reached from 8 away: only monsters that close to the disk.
                        candidates
                            .retain(|c| (c.1 - CENTRE).length() <= LURE_RADIUS + RANGED_REACH);
                        (plan_tour(at, CENTRE, &candidates, n).into_iter())
                            .map(|id| Stop {
                                id,
                                reach: RANGED_REACH,
                            })
                            .collect()
                    }
                    Planner::Casts => {
                        let ids: Vec<u32> = candidates.iter().map(|c| c.0).collect();
                        let skills = (now >= p.blast_ready, now >= p.pierce_ready);
                        let fence = (LURE_RADIUS, !strong_ok);
                        plan_casts((at, CENTRE), monsters, &ids, n, skills, fence)
                    }
                };
                lure.planned_at = now;
            }
            let next = lure.tour.first().map(|s| (find(s.id), s.reach));
            let tagged_some = lure.casts() > lure.left_casts;
            match next {
                Some((Some(m), reach)) if !back => {
                    lure.scout = None;
                    hunt(&mut p.body, m.body.position, reach, RUN_SPEED, dt, rocks);
                }
                _ if back || tagged_some => {
                    lure.stage = LureStage::Back;
                    lure.tour.clear();
                    lure.scout = None;
                    hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt, rocks);
                }
                _ => {
                    // Nothing in sight and nothing tagged yet: scout the band, away from
                    // another lurer that is out.
                    if lure.scout.is_none_or(|spot| (spot - at).length() <= HOME) {
                        let other = (lure.last_seen.iter().enumerate())
                            .filter(|(j, _)| *j != me)
                            .find_map(|(_, seen)| {
                                seen.filter(|(_, t)| now - t <= SEEN_MEMORY)
                                    .map(|(at, _)| at)
                            });
                        lure.scout = Some(scout_spot(rng, other));
                    }
                    let spot = lure.scout.expect("a scouting spot");
                    hunt(&mut p.body, spot, HOME, RUN_SPEED, dt, rocks);
                }
            }
        }
        LureStage::Back => {
            if hunt(&mut p.body, CENTRE, HOME, RUN_SPEED, dt, rocks) == Gait::Standing {
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
    (lurers, strong_ok): (&[Option<Vec2>], bool),
    counts: &mut Lure,
    now: f32,
) -> Vec<usize> {
    let untagged = |hits: &[usize]| hits.iter().filter(|&&i| !engaged(&monsters[i])).count();
    // With strong monsters off limits, no area cast that would catch one.
    let allowed =
        |hits: &[usize]| strong_ok || hits.iter().all(|&i| !monsters[i].rarity.is_strong());
    let at = p.body.position;
    let candidates = (monsters.iter().enumerate())
        .filter(|(_, m)| !engaged(m) && owner(m.body.position, lurers) == Some(lurer))
        .filter(|(_, m)| strong_ok || !m.rarity.is_strong())
        .map(|(i, m)| (i, (m.body.position - at).length()));
    let (mut blast, mut pierce): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (i, d) in candidates {
        let centre = monsters[i].body.position;
        if d <= RANGED_REACH {
            let hits = in_blast(centre, LURE_BLAST, monsters);
            if allowed(&hits) && untagged(&hits) > untagged(&blast) {
                blast = hits;
            }
        }
        if d <= PIERCE_LENGTH {
            let hits = along_pierce(at, centre, monsters);
            if allowed(&hits) && untagged(&hits) > untagged(&pierce) {
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
    /// The team round the centre: the two lurers 1.5 either side of it, the four dps 4 out
    /// at the diagonals, the buffers 2.5 above and below; `DEFAULT_MONSTERS` spawned.
    fn default() -> Self {
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
            heal_ready: 0.0,
            group_ready: 0.0,
            guard_ready: 0.0,
            guard_until: 0.0,
        };
        let eighth = TAU / 8.0;
        let team = vec![
            player(Role::Lurer, CENTRE + Vec2::new(-1.5, 0.0)),
            player(Role::Lurer, CENTRE + Vec2::new(1.5, 0.0)),
            player(Role::Dps, around(CENTRE, 4.0, eighth)),
            player(Role::Dps, around(CENTRE, 4.0, 3.0 * eighth)),
            player(Role::Dps, around(CENTRE, 4.0, 5.0 * eighth)),
            player(Role::Dps, around(CENTRE, 4.0, 7.0 * eighth)),
            player(Role::HpBuffer, CENTRE + Vec2::new(0.0, 2.5)),
            player(Role::MpBuffer, CENTRE + Vec2::new(0.0, -2.5)),
        ];
        let mut field = Self {
            team,
            monsters: Vec::new(),
            obstacles: Rocks::default(),
            wanted: DEFAULT_MONSTERS,
            respawns: Vec::new(),
            kills: 0,
            heals: 0,
            group_heals: 0,
            guards: 0,
            next_id: 0,
            deaths: 0,
            lures: vec![Lure::new(); LURERS],
            bars: VecDeque::new(),
            settings: Settings::default(),
            rng: fastrand::Rng::with_seed(SEED),
            time: 0.0,
        };
        field.spawn_missing();
        field.obstacles = place_rocks(DEFAULT_OBSTACLES, &field.nests());
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

    pub fn obstacles(&self) -> &Rocks {
        &self.obstacles
    }

    /// Every nest: the living monsters' and those waiting to respawn.
    fn nests(&self) -> Vec<Vec2> {
        (self.monsters.iter().map(|m| m.spawn))
            .chain(self.respawns.iter().map(|r| r.1))
            .collect()
    }

    /// The party's mean HP in percent as the party window shows it, the dead at 0.
    pub fn party_hp(&self) -> f32 {
        let total: f32 = self.team.iter().map(|p| p.hp / MAX_HP * 100.0).sum();
        total / self.team.len() as f32
    }

    /// The party window's shortest bar: the lowest member's HP in percent, the dead at 0.
    pub fn lowest_hp(&self) -> f32 {
        self.bars_now().into_iter().fold(100.0, f32::min)
    }

    /// The most any member's HP fell over the last `TREND_WINDOW`, in points; 0 when nobody
    /// fell.
    pub fn worst_fall(&self) -> f32 {
        match (self.bars.front(), self.bars.back()) {
            (Some(first), Some(last)) => (first.iter().zip(last))
                .map(|(a, b)| a - b)
                .fold(0.0, f32::max),
            _ => 0.0,
        }
    }

    /// The dps work left in the dps disk: the HP of the monsters inside it at `DPS_RATE`, in
    /// seconds.
    pub fn disk_backlog(&self) -> f32 {
        (self.monsters.iter().filter(in_dps_disk))
            .map(|m| m.hp)
            .sum::<f32>()
            / DPS_RATE
    }

    /// The party window now: every member's HP as the game shows it, in tenths.
    fn bars_now(&self) -> Vec<f32> {
        self.team
            .iter()
            .map(|p| window_tenth(p.hp, MAX_HP))
            .collect()
    }

    /// Spawns the missing monsters, then moves every body: a player runs inside its role's
    /// disk; a monster with a player inside its big disk closes on the nearest one (`hunt`),
    /// otherwise keeps inside `IDLE_RADIUS` of its spawn at the gait it drew for the next
    /// `GAIT_MIN_S`-`GAIT_MAX_S` seconds, each with its whole body inside.
    pub fn update(&mut self, dt: f32) {
        // The dps and buffers, and a monster fighting inside the dps disk, mind no rocks.
        let no_rocks = Rocks::default();
        self.time += dt;
        self.bury();
        self.spawn_missing();
        let snap: Vec<Snap> = (self.team[..LURERS].iter())
            .map(|p| Snap {
                at: p.body.position,
                alive: !p.is_dead(),
                hp_ok: p.hp >= LEAVE_HP * MAX_HP,
                speed: p.body.speed,
            })
            .collect();
        let chasers: Vec<usize> = (0..LURERS)
            .map(|l| self.monsters.iter().filter(|m| m.target == Some(l)).count())
            .collect();
        // Each lurer reads the disk's backlog with the disk in sight, from the HP it knows
        // (max HP less the damage it saw), and adds what chases it from outside the disk.
        let now = self.time;
        let backlogs: Vec<f32> = (0..LURERS)
            .map(|l| {
                let lure = &self.lures[l];
                let known_disk: f32 = (self.monsters.iter().filter(in_dps_disk))
                    .map(|m| lure.known_hp(m))
                    .sum::<f32>()
                    / DPS_RATE;
                let chasing_hp: f32 = (self.monsters.iter())
                    .filter(|m| m.target == Some(l) && !in_dps_disk(m))
                    .map(|m| lure.known_hp(m))
                    .sum();
                let at = self.team[l].body.position;
                let lure = &mut self.lures[l];
                if (at - CENTRE).length() + DPS_RADIUS <= LURER_SIGHT {
                    lure.backlog_seen = Some((known_disk, now));
                }
                lure.known_backlog = lure.backlog(now, chasing_hp);
                lure.known_backlog
            })
            .collect();
        // The party window: the shortest bar now and the worst fall over the trend window.
        let window = (TREND_WINDOW / dt).round() as usize + 1;
        self.bars.push_back(self.bars_now());
        while self.bars.len() > window {
            self.bars.pop_front();
        }
        let (lowest, fall) = (self.lowest_hp(), self.worst_fall());
        let healer_ok = (self.team.iter())
            .filter(|p| p.role == Role::HpBuffer)
            .all(|p| !p.is_dead() && p.mp >= HEALER_MP);
        let cap = self.settings.backlog_cap;
        let party_ok = healer_ok && lowest >= RISKY_HP && fall <= RISKY_DROP;
        let strong_ok: Vec<bool> = (backlogs.iter())
            .map(|&b| (cap == 0.0 || b <= cap) && party_ok)
            .collect();
        let halt = lowest < HALT_HP || fall > HALT_DROP;
        let rng = &mut self.rng;
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
                        heal_ready: self.time,
                        group_ready: self.time,
                        guard_ready: self.time,
                        guard_until: 0.0,
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
            // A lurer's view of the lurers: the other only while nearby.
            let view = match p.role {
                Role::Lurer => observe(i, &snap),
                _ => Vec::new(),
            };
            let positions: Vec<Option<Vec2>> = view.iter().map(|o| o.seen).collect();
            p.target = match p.role {
                Role::Dps => nearest_monster_in_disk(p.body.position, &self.monsters),
                Role::Lurer => {
                    untagged_in_reach(i, p.body.position, &self.monsters, &positions, strong_ok[i])
                }
                _ => None,
            };
            if p.role == Role::Lurer {
                let (monsters, now) = (&self.monsters, self.time);
                let lure = &mut self.lures[i];
                let seen = (positions.as_slice(), strong_ok[i]);
                let hit = lure_cast(p, i, p.target, monsters, seen, lure, now);
                tags.extend(hit.into_iter().map(|t| (i, t)));
                let trip = Trip {
                    settings: &self.settings,
                    monsters,
                    lurers: &view,
                    chasers: chasers[i],
                    backlog: backlogs[i],
                    strong_ok: strong_ok[i],
                    halt,
                    now,
                    dt,
                };
                lure_step(p, i, lure, &trip, rng, &self.obstacles);
                continue;
            }
            let Some(t) = p.target else {
                let patch = Patch {
                    centre: CENTRE,
                    radius: p.role.radius(),
                    rocks: &no_rocks,
                };
                roam(
                    &mut p.body,
                    Gait::Running,
                    RUN_SPEED,
                    &mut p.spot,
                    patch,
                    dt,
                    rng,
                );
                continue;
            };
            let at = self.monsters[t].body.position;
            if hunt(&mut p.body, at, RANGED_REACH, RUN_SPEED, dt, &no_rocks) == Gait::Standing
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
        let mut landed: Vec<(usize, f32)> = Vec::new();
        for (lurer, t) in tags {
            let m = &mut self.monsters[t];
            m.hp -= TAG_DAMAGE;
            *m.threat.entry(lurer).or_insert(0.0) += TAG_DAMAGE;
            landed.push((t, TAG_DAMAGE));
        }
        for (shooter, at) in hits {
            for t in in_blast(at, AOE_BLAST, &self.monsters) {
                let m = &mut self.monsters[t];
                m.hp -= AOE_DAMAGE;
                *m.threat.entry(shooter).or_insert(0.0) += AOE_DAMAGE;
                landed.push((t, AOE_DAMAGE));
            }
        }
        // A lurer sees the hits that land within its sight and keeps a tally per monster.
        for (p, lure) in self.team[..LURERS].iter().zip(&mut self.lures) {
            if p.is_dead() {
                continue;
            }
            for &(t, damage) in &landed {
                let m = &self.monsters[t];
                if (m.body.position - p.body.position).length() <= LURER_SIGHT {
                    lure.see_hit(m.id, damage);
                }
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
                // Rocks lie outside the dps disk; inside it a monster fights unhindered.
                let rocks = if (at - CENTRE).length() > DPS_RADIUS {
                    &self.obstacles
                } else {
                    &no_rocks
                };
                m.gait = hunt(&mut m.body, players[i], reach, speed, dt, rocks);
                // A fresh gait draw as soon as the chase ends.
                m.until = self.time;
                let in_reach = (players[i] - m.body.position).length() <= reach;
                if in_reach && self.time >= m.bite_ready {
                    m.bite_ready = self.time + BITE_COOLDOWN;
                    let guard = if self.time < self.team[i].guard_until {
                        GUARD_FACTOR
                    } else {
                        1.0
                    };
                    self.team[i].hp -= self.settings.bite * m.rarity.damage_multiplier() * guard;
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
            let patch = Patch {
                centre: m.spawn,
                radius: IDLE_RADIUS,
                rocks: &self.obstacles,
            };
            roam(&mut m.body, m.gait, speed, &mut m.spot, patch, dt, rng);
        }
        self.count_deliveries();
    }

    /// The trip metrics after a step, per lurer: a monster chasing a lurer that enters the
    /// dps disk is delivered (once, to that lurer), and so is one a dps's damage takes over
    /// within the blast's reach of the disk (a pack's trailing monster, splashed at the edge);
    /// one that stops chasing it anywhere else outside the disk is lost; each lurer's steps
    /// inside the disk are counted.
    fn count_deliveries(&mut self) {
        let alive: Vec<bool> = self.team[..LURERS].iter().map(|p| !p.is_dead()).collect();
        for m in &mut self.monsters {
            let chasing = m.target.filter(|&t| t < LURERS);
            let out = (m.body.position - CENTRE).length();
            let inside = out <= DPS_RADIUS;
            let splashed = m.target.is_some_and(|t| t >= LURERS) && out <= DPS_RADIUS + AOE_BLAST;
            let mut deliver = |m: &mut Monster, l: usize| {
                if !m.delivered {
                    m.delivered = true;
                    self.lures[l].delivered += 1;
                }
            };
            if let (Some(l), true) = (chasing, inside) {
                deliver(m, l);
            }
            if let Some(l) = m.was_chasing_lurer {
                if chasing != Some(l) && !inside && alive[l] {
                    if splashed {
                        deliver(m, l);
                    } else {
                        self.lures[l].lost += 1;
                    }
                }
            }
            m.was_chasing_lurer = chasing;
        }
        for (p, l) in self.team[..LURERS].iter().zip(&mut self.lures) {
            l.steps += 1;
            if (p.body.position - CENTRE).length() <= DPS_RADIUS {
                l.steps_in_disk += 1;
            }
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
            "backlog_cap" => s.backlog_cap = BACKLOG_CAP.clamp(value),
            "obstacles" => {
                s.obstacles = OBSTACLES.clamp(value) as usize;
                self.obstacles = place_rocks(self.settings.obstacles, &self.nests());
            }
            _ => {}
        }
    }

    /// Buffer `b` (nothing for other roles) casts its buff when ready and affordable: on the
    /// living member within its ranged reach with the lowest bar under the threshold and no
    /// such buff running, the buffer itself included.
    fn cast_buff(&mut self, b: usize) {
        type Read = fn(&Player) -> f32;
        let buffer = &self.team[b];
        let ready = !buffer.is_dead() && self.time >= buffer.skill_ready;
        if buffer.role == Role::HpBuffer && ready {
            // The most urgent first: a heal, a group heal, a guard; else the HP buff below.
            if self.cast_heal(b) || self.cast_group_heal(b) || self.cast_guard(b) {
                return;
            }
        }
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

    /// The living members within the buffer's ranged reach, nearest-lowest-HP order left to
    /// the caller: (index, HP share).
    fn members_in_reach(&self, b: usize) -> Vec<(usize, f32)> {
        let at = self.team[b].body.position;
        (self.team.iter().enumerate())
            .filter(|(_, p)| !p.is_dead() && (p.body.position - at).length() <= RANGED_REACH)
            .map(|(i, p)| (i, p.hp / MAX_HP))
            .collect()
    }

    /// Marks a cast by buffer `b`: its MP, the one-cast-a-second gate.
    fn spend(&mut self, b: usize, mp: f32) {
        self.team[b].mp -= mp;
        self.team[b].skill_ready = self.time + BUFF_COOLDOWN;
    }

    /// A heal (`HEAL_HP` at once) on the member in reach lowest under `HEAL_AT`.
    fn cast_heal(&mut self, b: usize) -> bool {
        let buffer = &self.team[b];
        if self.time < buffer.heal_ready || buffer.mp < HEAL_MP {
            return false;
        }
        let member = (self.members_in_reach(b).into_iter())
            .filter(|&(_, share)| share < HEAL_AT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        let Some(i) = member else {
            return false;
        };
        self.team[i].hp = (self.team[i].hp + HEAL_HP).min(MAX_HP);
        self.team[b].heal_ready = self.time + HEAL_COOLDOWN;
        self.spend(b, HEAL_MP);
        self.heals += 1;
        true
    }

    /// A group heal (`GROUP_HEAL_HP` to every member in reach) when `GROUP_HEAL_MEMBERS` or
    /// more of them are under `GROUP_HEAL_AT` percent.
    fn cast_group_heal(&mut self, b: usize) -> bool {
        let buffer = &self.team[b];
        if self.time < buffer.group_ready || buffer.mp < GROUP_HEAL_MP {
            return false;
        }
        let in_reach = self.members_in_reach(b);
        let hurt = (in_reach.iter())
            .filter(|&&(_, fill)| fill * 100.0 < GROUP_HEAL_AT)
            .count();
        if hurt < GROUP_HEAL_MEMBERS {
            return false;
        }
        for (i, _) in in_reach {
            self.team[i].hp = (self.team[i].hp + GROUP_HEAL_HP).min(MAX_HP);
        }
        self.team[b].group_ready = self.time + GROUP_HEAL_COOLDOWN;
        self.spend(b, GROUP_HEAL_MP);
        self.group_heals += 1;
        true
    }

    /// A guard (`GUARD_DURATION` of bites times `GUARD_FACTOR`) on the unguarded member in
    /// reach that a monster is attacking, the lowest HP first.
    fn cast_guard(&mut self, b: usize) -> bool {
        let buffer = &self.team[b];
        if self.time < buffer.guard_ready || buffer.mp < GUARD_MP {
            return false;
        }
        let attacked = |i: usize| self.monsters.iter().any(|m| m.target == Some(i));
        let member = (self.members_in_reach(b).into_iter())
            .filter(|&(i, _)| attacked(i) && self.time >= self.team[i].guard_until)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        let Some(i) = member else {
            return false;
        };
        self.team[i].guard_until = self.time + GUARD_DURATION;
        self.team[b].guard_ready = self.time + GUARD_COOLDOWN;
        self.spend(b, GUARD_MP);
        self.guards += 1;
        true
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
        let nests: Vec<Vec2> = (self.monsters.iter())
            .filter(|m| m.hp <= 0.0)
            .map(|m| m.spawn)
            .collect();
        for m in self.monsters.iter().filter(|m| m.hp <= 0.0) {
            for lure in &mut self.lures {
                lure.damage_seen.remove(&m.id);
            }
        }
        self.monsters.retain(|m| m.hp > 0.0);
        self.kills += nests.len() as u32;
        let due = self.time + MONSTER_RESPAWN;
        self.respawns
            .extend(nests.into_iter().map(|nest| (due, nest)));
    }

    /// Spawns monsters until `wanted` are on the map, standing until their first gait draw;
    /// a killed monster's slot waits for its respawn time.
    fn spawn_missing(&mut self) {
        let now = self.time;
        let (due, waiting): (Vec<_>, Vec<_>) = self.respawns.iter().partition(|r| r.0 <= now);
        self.respawns = waiting;
        for (_, nest) in due {
            self.spawn_at(nest);
        }
        while self.monsters.len() + self.respawns.len() < self.wanted {
            let at = self.spawn_point();
            self.spawn_at(at);
        }
    }

    /// A monster of a drawn rarity, standing at `at`, its nest.
    fn spawn_at(&mut self, at: Vec2) {
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
            was_chasing_lurer: None,
            bite_ready: self.time,
        };
        monster.set_rarity(rarity);
        self.monsters.push(monster);
    }

    /// A random point of the map, an idle radius in from its edge and outside the dps disk
    /// (nothing appears on top of the team).
    fn spawn_point(&mut self) -> Vec2 {
        loop {
            let at = random_spot(CENTRE, MAP_RADIUS - IDLE_RADIUS, &mut self.rng);
            if (at - CENTRE).length() > DPS_RADIUS && self.obstacles.nest_room(at) {
                return at;
            }
        }
    }

    pub fn view(&self) -> FrameView {
        let (w, h) = (MAP_RADIUS, MAP_RADIUS);
        let corner = |x: f32, y: f32| CENTRE + Vec2::new(x, y);
        let mut lines = Vec::new();
        for p in &self.team {
            let at = p.body.position;
            lines.extend(bar(at, HP_BAR_LIFT, p.hp / MAX_HP, HP_BUFFER_BODY));
            lines.extend(bar(at, MP_BAR_LIFT, p.mp / MAX_MP, MP_BUFFER_BODY));
        }
        for m in &self.monsters {
            let at = m.body.position;
            lines.extend(bar(at, HP_BAR_LIFT, m.hp / m.max_hp, HP_BUFFER_BODY));
        }
        let mut circles = vec![circle_view(CENTRE, MAP_RADIUS, color::EDGE)];
        circles.extend(
            (self.obstacles.circles.iter())
                .map(|o| circle_view(o.center, o.radius, color::OBSTACLE)),
        );
        lines.extend(
            (self.obstacles.rectangles.iter())
                .flat_map(|w| w.edges().map(|(a, b)| line(a, b, color::OBSTACLE))),
        );
        for p in &self.team[..LURERS] {
            circles.push(circle_view(p.body.position, LURER_SIGHT, color::SIGHT));
        }
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
        // A chasing monster's line of sight, except once it fights inside the dps disk.
        let chasing_outside =
            |m: &&Monster| m.target.is_some() && (m.body.position - CENTRE).length() > DPS_RADIUS;
        for m in self.monsters.iter().filter(chasing_outside) {
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
            if self.time < p.guard_until {
                circles.push(circle_view(p.body.position, GUARD_RING, color::GUARD));
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
            (BUFF_RADIUS, format!("HP buffer {BUFF_RADIUS}")),
            (DPS_RADIUS, format!("dps and MP buffer {DPS_RADIUS}")),
            (LURE_RADIUS, format!("lurer {LURE_RADIUS}")),
        ] {
            circles.push(circle_view(CENTRE, r, color::RADIUS));
            labels.push(label(
                CENTRE + Vec2::new(0.0, -r - 1.0),
                &text,
                color::LABEL,
            ));
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
        // The three status lines (the team, lurer A, lurer B) stand bottom-left inside the
        // map's square: a landscape canvas shows 76 each way (`view_size` 152), so a line
        // anchored below 70 is cut off. A label hangs above and right of its anchor.
        labels.push(label(
            corner(-w, h - 9.0),
            format!(
                "{} monsters: {}; {chasing} chasing; {} kills, {} deaths; party HP {:.0}%, lowest {:.0}% (fell {:.0} in 5 s); HP buffer: {} heals, {} group heals, {} guards; disk backlog {:.0} s",
                self.monsters.len(),
                gaits.join(", "),
                self.kills,
                self.deaths,
                self.party_hp(),
                self.lowest_hp(),
                self.worst_fall(),
                self.heals,
                self.group_heals,
                self.guards,
                self.disk_backlog()
            ),
            color::LABEL,
        ));
        for (i, (p, lure)) in self.team[..LURERS].iter().zip(&self.lures).enumerate() {
            let name = ["A", "B"][i];
            let at = p.body.position + Vec2::new(-BAR_WIDTH / 2.0, -MP_BAR_LIFT - 0.6);
            labels.push(label(at, name, color::LABEL));
            labels.push(label(
                corner(-w, h - 5.0 + 4.0 * i as f32),
                format!(
                    "lurer {name} {:?}: {} trips, {} delivered, {} lost, {:.0}% in the dps disk; {} casts; {} sessions of {:.1} s, back with {:.0} HP (lowest {:.0}), {:.0} walked per delivered; knows {:.0} s of backlog",
                    lure.stage,
                    lure.trips,
                    lure.delivered,
                    lure.lost,
                    100.0 * lure.share_in_disk(),
                    lure.casts(),
                    lure.sessions,
                    lure.mean_session(),
                    lure.mean_hp_back(),
                    if lure.sessions > 0 { lure.hp_back_min } else { 0.0 },
                    lure.walked / lure.delivered.max(1) as f32,
                    lure.known_backlog
                ),
                color::LABEL,
            ));
            if lure.stage == LureStage::Out {
                if let (true, Some(spot)) = (lure.tour.is_empty(), lure.scout) {
                    lines.push(line(p.body.position, spot, color::PATH));
                }
                let mut from = p.body.position;
                for stop in &lure.tour {
                    if let Some(m) = self.monsters.iter().find(|m| m.id == stop.id) {
                        lines.push(line(from, m.body.position, color::PATH));
                        from = m.body.position;
                    }
                }
                lines.push(line(from, CENTRE, color::PATH));
            }
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
            view_size: 2.0 * MAP_RADIUS + 12.0,
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
                (LURER_BODY, "our lurers A and B"),
                (DPS_BODY, "dps (4)"),
                (
                    HP_BUFFER_BODY,
                    "HP buffer; the HP bar over every body; a ring: its buff running",
                ),
                (
                    MP_BUFFER_BODY,
                    "MP buffer; the MP bar over every player; a ring: its buff running",
                ),
                (color::GUARD, "guard: bites on the member do 20% less for 20 s"),
                (MONSTER_BODY, "monster"),
                (
                    color::RADIUS,
                    "training disks: each role's radius round the centre",
                ),
                (color::EDGE, "map edge: a disk of 60, the nests inside"),
                (
                    color::OBSTACLE,
                    "rocks: random circles and rectangles in the luring disk; a runner outside the dps disk steers round them 2 s ahead",
                ),
                (color::TRACK, "the empty part of a bar"),
                (
                    color::RANGED,
                    "big disk: a player's ranged reach 8 (attack, buffs), a monster's alarm 6",
                ),
                (color::MELEE, "melee reach 2: a monster's attack"),
                (
                    color::SIGHT,
                    "sight: a lurer sees monsters within 30; a chasing monster keeps its target within 35 (not shown inside the dps disk)",
                ),
                (color::PATH, "a lurer's planned tour: its stops, then home; or its scouting spot"),
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
                    "obstacles",
                    "Obstacles",
                    OBSTACLES,
                    self.settings.obstacles as f32,
                ),
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
                    "backlog_cap",
                    "Backlog cap, s (0: none)",
                    BACKLOG_CAP,
                    self.settings.backlog_cap,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steer_core::containment::room;

    /// Over `seconds` after a 10 s warm-up: the furthest any player's centre gets past the ring
    /// its body must stay inside (`role.radius() - BODY_RADIUS`), the furthest any monster gets
    /// from its spawn, and each player's mean and lowest speed.
    struct Measured {
        player_over: f32,
        monster_out: f32,
        mean_speed: [f32; 8],
        min_speed: [f32; 8],
        /// The share of steps each player spent inside the buffers' disk.
        in_buff_disk: [f32; 8],
    }

    fn measure(field: &mut TrainingField, seconds: f32) -> Measured {
        const WARM_UP: usize = 600;
        let mut m = Measured {
            player_over: f32::MIN,
            monster_out: 0.0,
            mean_speed: [0.0; 8],
            min_speed: [f32::MAX; 8],
            in_buff_disk: [0.0; 8],
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
            gaits.push(hunt(
                &mut body,
                target,
                MELEE_REACH,
                RUN_SPEED,
                1.0 / 60.0,
                &Rocks::default(),
            ));
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
        let bodies = 8 + field.monsters().len();
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
        // Lurer A stands 1.5 left of the centre; its bars are drawn round that x.
        let x = field.team()[0].body.position.x;
        let hp_bars: Vec<_> = (view.lines.iter())
            .filter(|l| l.color == HP_BUFFER_BODY)
            .collect();
        let mp_bars = view
            .lines
            .iter()
            .filter(|l| l.color == MP_BUFFER_BODY)
            .count();
        assert_eq!(hp_bars.len(), (8 + DEFAULT_MONSTERS) * BAR_STRANDS);
        assert_eq!(mp_bars, 8 * BAR_STRANDS);
        // Every bar lies on a full-width track drawn before it.
        let tracks: Vec<_> = (view.lines.iter())
            .filter(|l| l.color == color::TRACK)
            .collect();
        assert_eq!(tracks.len(), (8 + DEFAULT_MONSTERS + 8) * BAR_STRANDS);
        assert_eq!(tracks[0].to, [x + BAR_WIDTH / 2.0, -HP_BAR_LIFT]);
        let first_track = view.lines.iter().position(|l| l.color == color::TRACK);
        let first_fill = view.lines.iter().position(|l| l.color == HP_BUFFER_BODY);
        assert!(first_track < first_fill);
        // The lurer's half-full HP bar is half the bar's width, above its centre; its
        // strands sit one gap apart.
        let lurer = &hp_bars[0];
        assert_eq!(lurer.from, [x - BAR_WIDTH / 2.0, -HP_BAR_LIFT]);
        assert_eq!(lurer.to, [x, -HP_BAR_LIFT]);
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
                .filter(|c| c.color == color::SIGHT && c.radius == LOS_REACH)
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
        // A monster chasing inside the dps disk shows no line of sight either.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(4.0, 0.0));
        field.monsters[0].threat.insert(0, 1.0);
        step_with_parked_lurer(&mut field, CENTRE, 2);
        // (A dps hit may already have taken it over; it chases someone either way.)
        assert!(field.monsters()[0].target.is_some());
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
        field.param("obstacles", 0.0);
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
        // Six monsters in a row below the centre, 0.4 apart: every dps aims at the first
        // (lurer A leaves monsters inside the disk alone); the blasts cover all six and hit
        // the five nearest, the sixth untouched.
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        for (k, m) in field.monsters.iter_mut().enumerate().skip(1) {
            m.body = standing(Vec2::new(0.0, -4.0 - 0.4 * k as f32));
        }
        field.monsters.push(field.monsters[4].clone());
        field.monsters[5].body = standing(Vec2::new(0.0, -6.0));
        field.update(1.0 / 60.0);
        let hp: Vec<f32> = field.monsters().iter().map(|m| m.hp).collect();
        assert_eq!(hp, [60.0, 60.0, 60.0, 60.0, 60.0, 100.0]);
        assert_eq!((field.lures[0].blasts, field.lures[0].tags), (0, 0));
    }

    #[test]
    fn the_lurer_casts_the_skill_that_tags_the_most() {
        let den = Vec2::new(60.0, 40.0);
        // One monster 7 away: the normal attack.
        let mut field = quiet_field();
        field.monsters[0].body = standing(den + Vec2::new(0.0, -7.0));
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(
            (
                field.lures[0].tags,
                field.lures[0].blasts,
                field.lures[0].pierces
            ),
            (1, 0, 0)
        );
        // Three monsters spread sideways within 3 of one 7 away: the blast, all three tagged.
        let mut field = quiet_field();
        field.monsters[0].body = standing(den + Vec2::new(0.0, -7.0));
        field.monsters[1].body = standing(den + Vec2::new(2.5, -7.0));
        field.monsters[2].body = standing(den + Vec2::new(-2.5, -7.0));
        step_with_parked_lurer(&mut field, den, 2);
        assert_eq!(
            (
                field.lures[0].tags,
                field.lures[0].blasts,
                field.lures[0].pierces
            ),
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
            (
                field.lures[0].tags,
                field.lures[0].blasts,
                field.lures[0].pierces
            ),
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
        assert_eq!(view.vehicles[8].radius, 2.0);
        assert!(view.labels.iter().any(|l| l.text == "giant"));
        let bar = (view.lines.iter())
            .filter(|l| l.color == HP_BUFFER_BODY)
            .nth(8 * BAR_STRANDS)
            .unwrap();
        assert_eq!(bar.to[0] - bar.from[0], BAR_WIDTH / 2.0);
        step_with_parked_lurer(&mut field, den, 2);
        assert!((field.team()[0].hp - (MAX_HP - BITE * 3.0)).abs() < 0.1);
    }

    /// The three status lines stand bottom-left inside the map's square, the team's line on
    /// top, then lurer A, then B (a landscape canvas shows 66 each way; the first build hung
    /// them at 63, 67 and 71, and lurer B's line never showed).
    #[test]
    fn status_lines_sit_inside_the_map() {
        let view = TrainingField::default().view();
        let anchor = |start: &str| {
            let l = (view.labels.iter())
                .find(|l| l.text.starts_with(start))
                .unwrap_or_else(|| panic!("no label starting with {start:?}"));
            assert_eq!(l.at[0], -70.0);
            l.at[1]
        };
        let (team, a, b) = (anchor("15 monsters:"), anchor("lurer A"), anchor("lurer B"));
        assert!(team < a && a < b, "{team} {a} {b}");
        assert!(b <= 70.0 && team >= -70.0, "{team} {b}");
        assert!(2.0 * b + 12.0 <= view.view_size);
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
    fn the_lurer_leaves_for_work_in_its_disk_as_soon_as_its_hp_is_back() {
        // One monster 40 out (at the luring disk's edge, beyond sight): at full HP the trip
        // begins at once.
        let work = || field_with_a_monster_at(Vec2::new(40.0, 0.0));
        let mut field = work();
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].trips, 1);
        // Nothing in sight: it leaves all the same and scouts the band beyond its sight,
        // never past the luring disk.
        let mut field = quiet_field();
        let mut furthest = 0.0f32;
        for _ in 0..600 {
            field.update(1.0 / 60.0);
            furthest = furthest.max((field.team()[0].body.position - CENTRE).length());
        }
        assert_eq!(
            (field.lures[0].stage, field.lures[0].trips),
            (LureStage::Out, 1)
        );
        assert!(field.lures[0].scout.is_some());
        assert!(
            furthest > LURER_SIGHT && furthest <= LURE_RADIUS,
            "{furthest}"
        );
        // At 85 HP (under the buff threshold) it waits for the HP buff (+4) and regeneration
        // (+1) to bring it to 90: 1 s.
        let mut field = work();
        field.team[0].hp = 85.0;
        let mut left_at = None;
        for step in 0..300 {
            field.update(1.0 / 60.0);
            if left_at.is_none() && field.lures[0].trips > 0 {
                left_at = Some(step);
            }
        }
        assert!(matches!(left_at, Some(55..=65)), "{left_at:?}");
        // Home with a monster still chasing it (parked outside the disk, so the dps never
        // take it): no new trip until it is handed off; the moment nothing chases, it goes.
        let mut field = work();
        field.monsters[0].body = standing(Vec2::new(22.0, 0.0));
        field.monsters[0].spawn = field.monsters[0].body.position;
        field.monsters[0].gait = Gait::Standing;
        field.monsters[0].until = f32::MAX;
        field.monsters[0].threat.insert(0, 1.0);
        field.monsters[0].target = Some(0);
        field.monsters[1].body = standing(Vec2::new(-40.0, 0.0));
        field.monsters[1].spawn = field.monsters[1].body.position;
        for _ in 0..120 {
            field.update(1.0 / 60.0);
            field.monsters[0].body = standing(Vec2::new(22.0, 0.0));
        }
        assert_eq!(field.monsters()[0].target, Some(0));
        assert_eq!(field.lures[0].trips, 0);
        field.monsters[0].threat.clear();
        field.monsters[0].target = None;
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].trips, 1);
        // With four party monsters (4,000 HP, 150 s of dps work) parked in the dps disk the
        // lurer stays at a cap of 30 s; with the cap off, not.
        let parked = || {
            let mut field = work();
            field.param("backlog_cap", 30.0);
            for (k, m) in field.monsters.iter_mut().skip(1).take(4).enumerate() {
                m.body = standing(Vec2::new(7.0, k as f32 * 2.0 - 3.0));
                m.spawn = m.body.position;
                m.set_rarity(Rarity::PartyNormal);
            }
            field
        };
        let mut field = parked();
        for _ in 0..180 {
            field.update(1.0 / 60.0);
        }
        assert_eq!(
            (field.lures[0].stage, field.lures[0].trips),
            (LureStage::Centre, 0)
        );
        let mut field = parked();
        field.param("backlog_cap", 0.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].trips, 1);
    }

    #[test]
    fn the_lurer_may_tag_past_its_disk_when_a_skill_reaches_from_inside() {
        // From 35 out: a monster at 50 is a normal attack from 35 (inside the disk of 40);
        // one at 58 is not (43), and the pierce (12, shorter than the normal attack's 15)
        // reaches nothing the normal attack does not.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(50.0, 0.0));
        field.monsters[1].body = standing(Vec2::new(58.0, 0.0));
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let from = (Vec2::new(35.0, 0.0), CENTRE);
        let plan = |ids: &[u32], skills| {
            plan_casts(from, field.monsters(), ids, 1, skills, (LURE_RADIUS, false))
        };
        assert_eq!(
            plan(&ids[..1], (false, false)),
            [Stop {
                id: ids[0],
                reach: RANGED_REACH
            }]
        );
        assert!(plan(&ids[1..2], (false, false)).is_empty());
        assert!(plan(&ids[1..2], (false, true)).is_empty());
        // The sight: from the centre the lurer lists only monsters within 30.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(29.0, 0.0));
        field.monsters[1].body = standing(Vec2::new(31.0, 0.0));
        let seen = tour_candidates(0, CENTRE, field.monsters(), &[Some(CENTRE), None], true);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, field.monsters()[0].id);
    }

    #[test]
    fn each_lurer_takes_the_monsters_nearer_to_it() {
        let lurers = [Some(Vec2::new(-1.5, 0.0)), Some(Vec2::new(1.5, 0.0))];
        assert_eq!(owner(Vec2::new(10.0, 0.0), &lurers), Some(1));
        assert_eq!(owner(Vec2::new(-10.0, 0.0), &lurers), Some(0));
        // A tie goes to A; with B dead everything is A's.
        assert_eq!(owner(Vec2::new(0.0, 5.0), &lurers), Some(0));
        assert_eq!(owner(Vec2::new(10.0, 0.0), &[lurers[0], None]), Some(0));
        // A pair of monsters 20 to the left and a pair 20 to the right: after ten seconds A
        // has tagged only the left pair and B only the right, nobody both.
        let mut field = quiet_field();
        let spots = [(-20.0, 3.0), (-20.0, -3.0), (20.0, 3.0), (20.0, -3.0)];
        let mut side = HashMap::new();
        for (m, &(x, y)) in field.monsters.iter_mut().zip(&spots) {
            m.body = standing(Vec2::new(x, y));
            m.spawn = m.body.position;
            side.insert(m.id, x < 0.0);
        }
        // Checked every step: a tagged pair is delivered, killed and reborn untagged at its
        // nest within the ten seconds.
        let mut tagged: [std::collections::HashSet<u32>; 2] = Default::default();
        for _ in 0..600 {
            field.update(1.0 / 60.0);
            for m in field.monsters() {
                let (by_a, by_b) = (m.threat.contains_key(&0), m.threat.contains_key(&1));
                assert!(!(by_a && by_b), "monster {} tagged by both", m.id);
                if let Some(&left) = side.get(&m.id) {
                    assert!(!by_a || left, "A tagged a right-hand monster");
                    assert!(!by_b || !left, "B tagged a left-hand monster");
                }
                if by_a {
                    tagged[0].insert(m.id);
                }
                if by_b {
                    tagged[1].insert(m.id);
                }
            }
        }
        assert!(!tagged[0].is_empty() && !tagged[1].is_empty(), "{tagged:?}");
    }

    #[test]
    fn a_lurer_out_of_sight_does_not_count_for_the_other() {
        // Lurer A parked 45 out: B, at the centre, cannot see it, so a monster at 25 (nearer A)
        // is B's, and B tags it within a few seconds; A, 20 from it, never does.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(25.0, 0.0));
        let a = Vec2::new(45.0, 0.0);
        step_with_parked_lurer(&mut field, a, 300);
        let m = &field.monsters()[0];
        assert!(
            m.threat.contains_key(&1) && !m.threat.contains_key(&0),
            "{:?}",
            m.threat
        );
        // A parked 28 out: in B's sight, nearer the monster, so it is A's: A tags it, B not.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(25.0, 0.0));
        step_with_parked_lurer(&mut field, Vec2::new(28.0, 0.0), 300);
        let m = &field.monsters()[0];
        assert!(
            m.threat.contains_key(&0) && !m.threat.contains_key(&1),
            "{:?}",
            m.threat
        );
        // What B sees: with A 40 out, only itself; with A 25 out, both.
        let snap = |ax: f32| {
            vec![
                Snap {
                    at: Vec2::new(ax, 0.0),
                    alive: true,
                    hp_ok: true,
                    speed: 0.0,
                },
                Snap {
                    at: Vec2::new(1.5, 0.0),
                    alive: true,
                    hp_ok: true,
                    speed: 0.0,
                },
            ]
        };
        let far = observe(1, &snap(40.0));
        assert!(far[0].seen.is_none() && far[0].hp_ok && far[1].seen.is_some());
        let near = observe(1, &snap(25.0));
        assert_eq!(near[0].seen, Some(Vec2::new(25.0, 0.0)));
    }

    #[test]
    fn the_second_lurer_leaves_once_the_first_is_fifteen_out_or_after_the_timeout() {
        // Work on both sides, 31 out (A's stop, 15 short, lies 16 out): A leaves at once, B
        // when A is 15 out (about 1.5 s at a run of 11).
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(-31.0, 0.0));
        field.monsters[1].body = standing(Vec2::new(31.0, 0.0));
        field.update(1.0 / 60.0);
        assert_eq!((field.lures[0].trips, field.lures[1].trips), (1, 0));
        let mut left = None;
        for step in 1..600 {
            field.update(1.0 / 60.0);
            if field.lures[1].trips > 0 {
                let a_out = (field.team()[0].body.position - CENTRE).length();
                left = Some((step, a_out));
                break;
            }
        }
        let (step, a_out) = left.expect("B leaves");
        assert!((60..300).contains(&step), "{step}");
        assert!(a_out >= WATCH_DISTANCE - RUN_SPEED / 60.0, "{a_out}");
        // A held on the centre (parked every step): B waits the timeout, then goes.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(-31.0, 0.0));
        field.monsters[1].body = standing(Vec2::new(31.0, 0.0));
        let mut left = None;
        for step in 0..600 {
            step_with_parked_lurer(&mut field, CENTRE, 1);
            if field.lures[1].trips > 0 {
                left = Some(step);
                break;
            }
        }
        let step = left.expect("B leaves on the timeout") as f32 / 60.0;
        assert!(
            (WATCH_TIMEOUT..WATCH_TIMEOUT + 0.2).contains(&step),
            "{step}"
        );
    }

    #[test]
    fn the_backlog_holds_the_lurers_from_the_cap_up() {
        // Work parked 6 from the centre, a normal monster 25 out for the trip, the cap at
        // 30 s. Two normals (200 HP, 7.5 s of dps work): A leaves at once. A party normal
        // (1,000 HP, 37.5 s): both lurers wait; at a cap of 60 s, or with the cap off, A
        // leaves at once.
        let parked = |rarity: Rarity, count: usize| {
            let mut field = quiet_field();
            field.param("backlog_cap", 30.0);
            for (k, m) in field.monsters.iter_mut().take(count).enumerate() {
                m.body = standing(Vec2::new(6.0, 2.0 * k as f32));
                m.set_rarity(rarity);
            }
            field.monsters[2].body = standing(Vec2::new(-25.0, 0.0));
            field
        };
        let mut field = parked(Rarity::Normal, 2);
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].trips, 1);
        let mut field = parked(Rarity::PartyNormal, 1);
        for _ in 0..180 {
            field.update(1.0 / 60.0);
        }
        assert_eq!((field.lures[0].trips, field.lures[1].trips), (0, 0));
        assert!(field.monsters()[0].hp < field.monsters()[0].max_hp);
        for cap in [60.0, 0.0] {
            let mut field = parked(Rarity::PartyNormal, 1);
            field.param("backlog_cap", cap);
            field.update(1.0 / 60.0);
            assert_eq!(field.lures[0].trips, 1, "cap {cap}");
        }
        // The lurer's own reading: what it saw less the time since, plus what chases it (a
        // giant's 2,000 HP is 75 s at the four dps's 26.7 a second).
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
        let lure = Lure {
            backlog_seen: Some((40.0, 0.0)),
            ..Lure::new()
        };
        assert!(near(lure.backlog(5.0, 0.0), 35.0));
        assert!(near(lure.backlog(50.0, 0.0), 0.0));
        assert!(near(lure.backlog(5.0, 2000.0), 110.0));
        assert!(near(Lure::new().backlog(5.0, 100.0), 3.75));
    }

    /// Parks both lurers before and after every step (A at `a`, B at `b`).
    fn step_with_parked_lurers(field: &mut TrainingField, a: Vec2, b: Vec2, steps: usize) {
        for _ in 0..steps {
            field.team[0].body = standing(a);
            field.team[1].body = standing(b);
            field.update(1.0 / 60.0);
            field.team[0].body = standing(a);
            field.team[1].body = standing(b);
        }
    }

    #[test]
    fn a_lurer_knows_a_monsters_hp_only_from_the_hits_it_saw() {
        // A monster 4 below the centre: the four dps blast it in the first step (40 off its
        // 100). Lurer A at the centre sees the hits; B, parked 40 east (the monster 40.2
        // away, beyond its sight of 30), does not.
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        step_with_parked_lurers(&mut field, CENTRE, Vec2::new(40.0, 0.0), 1);
        let m = &field.monsters()[0];
        assert_eq!(m.hp, 60.0);
        assert_eq!(field.lures[0].known_hp(m), 60.0);
        assert_eq!(field.lures[1].known_hp(m), 100.0);
        // B comes to 10 east: the disk is in its sight and it reads the backlog from what
        // it knows, the whole 100 HP (3.75 s at the four dps's 26.7 a second); A reads the
        // 60 it saw (2.25 s). No blast lands within the 1.5 s cooldown.
        step_with_parked_lurers(&mut field, CENTRE, Vec2::new(10.0, 0.0), 2);
        let near = |a: f32, b: f32| (a - b).abs() < 0.05;
        let known = [field.lures[0].known_backlog, field.lures[1].known_backlog];
        assert!(near(known[0], 2.25) && near(known[1], 3.75), "{known:?}");
        // Killed, the monster's id leaves every tally (a respawn is a new monster).
        let id = field.monsters()[0].id;
        field.monsters[0].hp = 1.0;
        for _ in 0..120 {
            field.update(1.0 / 60.0);
        }
        assert!(field.monsters().iter().all(|m| m.id != id));
        assert!(field.lures.iter().all(|l| !l.damage_seen.contains_key(&id)));
    }

    #[test]
    fn the_party_window_shows_hp_in_tenths() {
        // The game's party packet carries each member's HP as tenths: 95% reads 90, 84
        // reads 80, 79 reads 70; so a fall from 95 to 84 reads 10 and one to 79 reads 20.
        let mut field = quiet_field();
        for (hp, lowest, fall) in [(95.0, 90.0, 0.0), (84.0, 80.0, 10.0), (79.0, 70.0, 20.0)] {
            field.team[2].hp = hp;
            field.update(1.0 / 60.0);
            assert_eq!(
                (field.lowest_hp(), field.worst_fall()),
                (lowest, fall),
                "hp {hp}"
            );
        }
        assert_eq!(window_tenth(100.0, MAX_HP), 100.0);
        assert_eq!(window_tenth(90.0, MAX_HP), 90.0);
        assert_eq!(window_tenth(0.0, MAX_HP), 0.0);
    }

    /// A giant and a normal monster 30 out on lurer A's side (in its sight, and 10 beyond the
    /// dps disk, past the alarm of a dps roaming to its edge), 6 apart (no cast takes both).
    fn risky_field() -> TrainingField {
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(-30.0, 0.0));
        field.monsters[0].set_rarity(Rarity::Giant);
        field.monsters[1].body = standing(Vec2::new(-30.0, 6.0));
        // Their nests where they stand and a frozen gait, so neither walks off to the far
        // corner while the lurer waits for the party.
        for m in field.monsters.iter_mut().take(2) {
            m.spawn = m.body.position;
            m.gait = Gait::Standing;
            m.until = f32::MAX;
        }
        field
    }

    /// Runs `steps`, recording which monsters (by id) lurer A tagged at any point.
    fn run_tracking_a(field: &mut TrainingField, steps: usize) -> std::collections::HashSet<u32> {
        let mut tagged = std::collections::HashSet::new();
        for _ in 0..steps {
            field.update(1.0 / 60.0);
            for m in field.monsters() {
                if m.threat.contains_key(&0) {
                    tagged.insert(m.id);
                }
            }
        }
        tagged
    }

    #[test]
    fn the_lurer_reads_the_disk_and_the_party_before_tagging_a_strong_monster() {
        // A calm party and an empty disk: A tags the normal and the giant within 8 s.
        let mut field = risky_field();
        let (giant, normal) = (field.monsters()[0].id, field.monsters()[1].id);
        let tagged = run_tracking_a(&mut field, 480);
        assert!(
            tagged.contains(&giant) && tagged.contains(&normal),
            "{tagged:?}"
        );
        // Two giants (14 apart, so no cast catches both) and the normal, the cap at 30 s: A
        // tags one giant and the normal and leaves the other giant alone, since the giant
        // chasing it is 75 s of dps work, above the cap.
        let mut field = risky_field();
        field.param("backlog_cap", 30.0);
        field.monsters[2].body = standing(Vec2::new(-26.0, -14.0));
        field.monsters[2].set_rarity(Rarity::Giant);
        let other = field.monsters()[2].id;
        let tagged = run_tracking_a(&mut field, 480);
        assert!(tagged.contains(&normal), "{tagged:?}");
        assert!(
            tagged.contains(&giant) != tagged.contains(&other),
            "{tagged:?}"
        );
        // The HP buffer at 0 MP: within 8 s the normal is tagged and the giant left alone;
        // with the MP buff (+3 a second, when the MP buffer is in reach) and regeneration the
        // healer is back over 50 by 15.5 s. A, held at the centre meanwhile (the band's
        // random scouting keeps it on the side away from B, out of sight of the giant, for
        // minutes), takes the giant on its next trip.
        let mut field = risky_field();
        field.team[6].mp = 0.0;
        let tagged = run_tracking_a(&mut field, 480);
        assert!(
            !tagged.contains(&giant) && tagged.contains(&normal),
            "{tagged:?}"
        );
        step_with_parked_lurer(&mut field, CENTRE, 8 * 60);
        assert!(field.team[6].mp >= HEALER_MP, "{}", field.team[6].mp);
        let tagged = run_tracking_a(&mut field, 10 * 60);
        assert!(tagged.contains(&giant), "{tagged:?}");
        // One dps falling 20 points at once (the party's mean 97.5%): no strong tag for the
        // 5 s window, and the giant is tagged on a later trip once the member has recovered.
        // (Regeneration and the buffs lift it again within seconds, so the window closes on
        // its own.)
        let mut field = risky_field();
        field.update(1.0 / 60.0);
        field.team[2].hp = 80.0;
        let (mut worst_fall, mut planned_giant, mut planned_normal) = (0.0f32, false, false);
        for _ in 0..60 {
            field.update(1.0 / 60.0);
            worst_fall = worst_fall.max(field.worst_fall());
            let tour = &field.lures[0].tour;
            planned_giant |= tour.iter().any(|s| s.id == giant);
            planned_normal |= tour.iter().any(|s| s.id == normal);
        }
        assert!(worst_fall > 15.0, "{worst_fall}");
        assert!(planned_normal && !planned_giant);
        let tagged = run_tracking_a(&mut field, (20.0 * 60.0) as usize);
        assert!(tagged.contains(&giant), "{tagged:?}");
        // One dps at 45% among seven at 100% (the mean 93%), the HP buffer out of MP for a
        // heal: no session starts; once the member is healed back over 50%, it does.
        let mut field = risky_field();
        field.team[2].hp = 45.0;
        field.team[6].mp = 0.0;
        for _ in 0..60 {
            field.update(1.0 / 60.0);
        }
        assert!(field.party_hp() > 90.0 && field.lowest_hp() < 50.0);
        assert_eq!(field.lures[0].trips, 0);
        for _ in 0..(15.0 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        assert!(field.lures[0].trips > 0);
        // One member fallen 31 points within the window: no session starts; fallen 29, it
        // does (A waits a step on its own HP first, so the window holds a full bar).
        for (hp, trips) in [(69.0, 0), (71.0, 1)] {
            let mut field = field_with_a_monster_at(Vec2::new(40.0, 0.0));
            field.team[0].hp = 80.0;
            field.update(1.0 / 60.0);
            field.team[2].hp = hp;
            field.team[0].hp = 100.0;
            field.update(1.0 / 60.0);
            assert_eq!(field.lures[0].trips, trips, "{hp}");
        }
        // The party window's numbers.
        let field = TrainingField::default();
        assert_eq!(field.party_hp(), 100.0);
        assert_eq!(field.lowest_hp(), 100.0);
        assert_eq!(field.worst_fall(), 0.0);
    }

    #[test]
    fn the_hp_buffer_heals_group_heals_and_guards_before_buffing() {
        // A dps at 40%: healed by 40 at once; a second one at 40% gets the HP buff while the
        // heal cools, and the heal 3 s later.
        let mut field = quiet_field();
        field.team[2].hp = 40.0;
        field.update(1.0 / 60.0);
        assert!(
            (field.team()[2].hp - 80.0).abs() < 0.1,
            "{}",
            field.team()[2].hp
        );
        assert_eq!(field.heals, 1);
        assert!((field.team()[6].mp - (MAX_MP - HEAL_MP)).abs() < 0.1);
        field.team[3].hp = 40.0;
        for _ in 0..70 {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[3].hp_buff_until > 0.0 && field.heals == 1);
        // (The HP buff lifts it over 50% before the heal cools.) Knocked down to 30 once the
        // cooldown is over and kept beside the buffer (both roam otherwise): healed at the
        // buffer's next cast.
        for _ in 0..120 {
            field.update(1.0 / 60.0);
        }
        field.team[3].hp = 30.0;
        for _ in 0..70 {
            field.update(1.0 / 60.0);
            field.team[3].body = standing(CENTRE + Vec2::new(0.5, 0.0));
            field.team[6].body = standing(CENTRE + Vec2::new(-0.5, 0.0));
        }
        assert_eq!(field.heals, 2);
        assert!(field.team()[3].hp > 65.0, "{}", field.team()[3].hp);
        // The whole party at 65%: a group heal of 20 on everyone in reach; lurer A, 40
        // away, misses it.
        let mut field = quiet_field();
        for p in &mut field.team {
            p.hp = 65.0;
        }
        field.team[0].body = standing(Vec2::new(40.0, 0.0));
        field.update(1.0 / 60.0);
        assert_eq!(field.group_heals, 1);
        assert!(
            field.team()[1..].iter().all(|p| p.hp > 84.0),
            "{:?}",
            field.team()[2].hp
        );
        assert!(field.team()[0].hp < 66.0);
        assert!((field.team()[6].mp - (MAX_MP - GROUP_HEAL_MP)).abs() < 0.1);
        // One dps at 60% among seven near full (the mean 95%): no group heal, the HP buff
        // instead; two dps at 60%: the group heal.
        for (hurt, group_heals) in [(1, 0), (2, 1)] {
            let mut field = quiet_field();
            for i in 2..2 + hurt {
                field.team[i].hp = 60.0;
            }
            field.update(1.0 / 60.0);
            assert_eq!(field.group_heals, group_heals, "{hurt} hurt");
            assert_eq!(field.heals, 0, "{hurt} hurt");
        }
        // A monster after dps 2: the guard goes on dps 2 once the monster has turned on it.
        let mut field = quiet_field();
        field.monsters[0].body = standing(Vec2::new(6.0, 0.0));
        field.monsters[0].threat.insert(2, 1.0);
        field.update(1.0 / 60.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.guards, 1);
        assert!(field.team()[2].guard_until > 0.0);
        assert!(field.view().circles.iter().any(|c| c.color == color::GUARD));
        // A guarded lurer bitten: 2.4 off instead of 3.
        let mut field = quiet_field();
        let den = Vec2::new(60.0, 40.0);
        field.monsters[0].body = standing(den + Vec2::new(0.0, 1.5));
        field.monsters[0].spawn = field.monsters[0].body.position;
        field.team[0].guard_until = 100.0;
        step_with_parked_lurer(&mut field, den, 2);
        // Against the literal cut, so a changed factor cannot move the expectation with it.
        let hp = field.team()[0].hp;
        assert!((hp - (MAX_HP - BITE * 0.8)).abs() < 0.1, "{hp}");
    }

    #[test]
    fn a_killed_monster_respawns_at_its_own_nest() {
        let mut field = TrainingField::default();
        let (nest, old) = (field.monsters()[0].spawn, field.monsters()[0].id);
        field.monsters[0].hp = 0.0;
        for _ in 0..(3.5 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        let back = (field.monsters().iter())
            .find(|m| (m.spawn - nest).length() < 1e-3)
            .expect("a monster at the nest");
        assert_ne!(back.id, old);
        assert_eq!(field.monsters().len(), DEFAULT_MONSTERS);
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
        assert!(top > 7.8 && top <= 8.0 + 1e-3, "{top}");
        // The params carry the values.
        let params = field.view().params;
        let value = |id: &str| params.iter().find(|p| p.id == id).unwrap().value;
        assert_eq!(
            (value("chasers"), value("bite"), value("backlog_cap")),
            (5.0, 10.0, 0.0)
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
        // Three monsters in a row to the left (lurer A's side), two far off elsewhere: A's
        // tour is the row, nearest first, and A sets off along it.
        let mut field = quiet_field();
        let spots = [
            (-25.0, 0.0),
            (-28.0, 0.0),
            (-31.0, 0.0),
            (40.0, 10.0),
            (0.0, -45.0),
        ];
        for (m, &(x, y)) in field.monsters.iter_mut().zip(&spots) {
            m.body = standing(Vec2::new(x, y));
            m.spawn = m.body.position;
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let tour = |field: &TrainingField| -> Vec<u32> {
            field.lures[0].tour.iter().map(|s| s.id).collect()
        };
        // The stops planner; the first step leaves the centre, the second plans.
        field.action("plan_stops");
        field.update(1.0 / 60.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].stage, LureStage::Out);
        assert_eq!(tour(&field), ids[..3]);
        // A second on: heading left along the tour (the low-speed rule bends the first
        // steps toward the starting heading), the tour unchanged by the re-plan.
        for _ in 0..60 {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[0].body.forward.x < -0.9);
        assert_eq!(tour(&field), ids[..3]);
        let tour_lines = (field.view().lines.iter())
            .filter(|l| l.color == color::PATH)
            .count();
        assert_eq!(tour_lines, 4);
        // It tags the row (the pierce takes all three at once, within 12), turns back and
        // is home: one session recorded, with its time and the HP it came back with.
        for _ in 0..60 * 30 {
            field.update(1.0 / 60.0);
            if field.lures[0].sessions > 0 {
                break;
            }
        }
        let l = &field.lures[0];
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
        // A cluster of three 30 to the right (within one blast) and a lone monster 25 to the
        // left; N = 3. With the blast: one stop at the cluster (15 out, 15 home). Without
        // either skill: three normal attacks, all on the cluster, none on the lone one.
        let mut field = quiet_field();
        let spots = [(30.0, 0.0), (30.0, 2.0), (30.0, -2.0), (-25.0, 0.0)];
        for (m, &(x, y)) in field.monsters.iter_mut().zip(&spots) {
            m.body = standing(Vec2::new(x, y));
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let plan = |field: &TrainingField, skills| {
            plan_casts(
                (Vec2::ZERO, CENTRE),
                field.monsters(),
                &ids[..4],
                3,
                skills,
                (LURE_RADIUS, false),
            )
        };
        let with_blast = plan(&field, (true, false));
        assert_eq!(with_blast.len(), 1);
        assert!(ids[..3].contains(&with_blast[0].id));
        assert_eq!(with_blast[0].reach, RANGED_REACH);
        let plain = plan(&field, (false, false));
        assert_eq!(plain.len(), 3);
        assert!(plain.iter().all(|s| ids[..3].contains(&s.id)));
        // Four in a radial row at 30, 33, 36, 39, all four wanted. In world units the normal
        // attack (15) reaches farther than the pierce (12), so the pierce never shortens a
        // run, only the casts: from 24 out a pierce toward the third takes the first three
        // and a normal attack the fourth, two casts for the same 48 there and back as four
        // normal attacks. (The kit's real shape, a taunt, an area roar and a sprint taunt, is
        // a later step.)
        let mut field = quiet_field();
        for (k, m) in field.monsters.iter_mut().take(4).enumerate() {
            m.body = standing(Vec2::new(30.0 + 3.0 * k as f32, 0.0));
        }
        let ids: Vec<u32> = field.monsters().iter().map(|m| m.id).collect();
        let pierce = plan_casts(
            (Vec2::ZERO, CENTRE),
            field.monsters(),
            &ids[..4],
            4,
            (false, true),
            (LURE_RADIUS, false),
        );
        assert_eq!(
            pierce,
            [
                Stop {
                    id: ids[2],
                    reach: PIERCE_LENGTH
                },
                Stop {
                    id: ids[3],
                    reach: RANGED_REACH
                }
            ]
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
            field.lures[0].stage = LureStage::Out;
            field.monsters[4].body = standing(Vec2::new(30.0, 0.0));
            field
        };
        let mut field = out();
        for m in field.monsters.iter_mut().take(BACK_CHASERS) {
            m.body = standing(Vec2::new(20.0, 0.0));
            m.threat.insert(0, 1.0);
        }
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].stage, LureStage::Out);
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].stage, LureStage::Back);
        // Out at 30% HP: Back at once.
        let mut field = out();
        field.team[0].hp = 30.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].stage, LureStage::Back);
        // Out, nothing more in sight but one monster tagged this trip and chasing: Back with
        // what follows, and home within a few steps.
        let mut field = quiet_field();
        field.lures[0].stage = LureStage::Out;
        field.lures[0].tags = 1;
        field.team[0].body = standing(Vec2::new(10.0, 0.0));
        field.monsters[0].body = standing(Vec2::new(13.0, 0.0));
        field.monsters[0].threat.insert(0, 1.0);
        field.update(1.0 / 60.0);
        assert_eq!(field.lures[0].stage, LureStage::Back);
        // Home within a few steps; at full HP it leaves again at once.
        let mut came_home = false;
        for _ in 0..240 {
            field.update(1.0 / 60.0);
            came_home |= field.lures[0].stage == LureStage::Centre || field.lures[0].trips >= 1;
        }
        assert!(came_home);
    }

    /// A pack's trailing monster that a dps's blast takes over at the disk's edge (within the
    /// blast's 2 outside the disk) counts delivered, not lost; one taken over further out is
    /// lost.
    #[test]
    fn a_monster_splashed_at_the_disks_edge_counts_delivered() {
        for (out, delivered, lost) in [(24.0, 1, 0), (26.0, 0, 1)] {
            let mut field = quiet_field();
            // The monster chases lurer A, parked 19 or more away (beyond A's tag reach).
            let park = Vec2::new(45.0, 0.0);
            field.monsters[0].body = standing(Vec2::new(out, 0.0));
            field.monsters[0].spawn = field.monsters[0].body.position;
            field.monsters[0].threat.insert(0, 1.0);
            field.monsters[0].target = Some(0);
            step_with_parked_lurer(&mut field, park, 1);
            assert_eq!(field.monsters[0].target, Some(0));
            // A dps's damage tops its threat table: it turns on the dps.
            field.monsters[0].threat.insert(2, 10.0);
            step_with_parked_lurer(&mut field, park, 1);
            assert_eq!(field.monsters[0].target, Some(2));
            assert_eq!(
                (field.lures[0].delivered, field.lures[0].lost),
                (delivered, lost),
                "at {out}"
            );
        }
    }

    /// The default field's rocks: 8, circles and rectangles, a circle's radius and a
    /// rectangle's half sides within 1.5..5, inside the luring disk, clear of the dps disk
    /// by 1, of every nest's idle disk by a body and of each other by 3 (by bounding radius);
    /// the slider only adds or removes rocks (the first 8 of 12 are the default 8); a new
    /// nest never lands within a rock's room.
    #[test]
    fn rocks_fill_the_luring_disk_clear_of_the_dps_disk_the_nests_and_each_other() {
        let mut field = TrainingField::default();
        let rocks = field.obstacles().clone();
        assert_eq!(rocks.len(), 8);
        assert!(!rocks.circles.is_empty() && !rocks.rectangles.is_empty());
        let nests: Vec<Vec2> = field.monsters().iter().map(|m| m.spawn).collect();
        for o in &rocks.circles {
            assert!((1.5..=5.0).contains(&o.radius), "{o:?}");
        }
        for w in &rocks.rectangles {
            let p = w.points();
            let (a, b) = ((p[0] - p[1]).length() / 2.0, (p[1] - p[2]).length() / 2.0);
            assert!(
                (1.5..=5.0).contains(&a) && (1.5..=5.0).contains(&b),
                "{w:?}"
            );
        }
        // Each rock as its centre and bounding radius.
        let bounds: Vec<(Vec2, f32)> = (rocks.circles.iter().map(|o| (o.center, o.radius)))
            .chain(rocks.rectangles.iter().map(|w| {
                let c = w.points().iter().fold(Vec2::ZERO, |s, &p| s + p) / 4.0;
                (c, (w.points()[0] - c).length())
            }))
            .collect();
        for (i, &(c, r)) in bounds.iter().enumerate() {
            let out = c.length();
            assert!(out + r <= 50.0 && out - r >= 11.0, "{c:?} {r}");
            for n in &nests {
                assert!((*n - c).length() >= r + 7.0, "{c:?} {n:?}");
            }
            for &(c2, r2) in &bounds[..i] {
                assert!((c2 - c).length() - r - r2 >= 3.0, "{c:?} {c2:?}");
            }
        }
        field.param("obstacles", 12.0);
        let more = field.obstacles().clone();
        assert_eq!(more.len(), 12);
        assert_eq!(&more.circles[..rocks.circles.len()], &rocks.circles[..]);
        assert_eq!(
            &more.rectangles[..rocks.rectangles.len()],
            &rocks.rectangles[..]
        );
        field.param("monsters", 30.0);
        for m in field.monsters() {
            assert!(field.obstacles().nest_room(m.spawn), "{:?}", m.spawn);
        }
        field.param("obstacles", 0.0);
        assert!(field.obstacles().is_empty());
    }

    /// How far `b` is from touching a rock; negative inside one.
    fn clearance(rocks: &Rocks, b: &Vehicle) -> f32 {
        let circles =
            (rocks.circles.iter()).map(|o| (b.position - o.center).length() - o.radius - b.radius);
        let slabs =
            (rocks.rectangles.iter()).map(|w| -room(&Wall::new(&w.offset(b.radius)), b.position));
        circles.chain(slabs).fold(f32::MAX, f32::min)
    }

    /// Runners steer round a rock in their path and still arrive, whether a circle of
    /// radius 3 or a 3 x 8 rectangle (square across the path, or turned): a monster chasing
    /// lurer A (parked at the centre) from 30 out passes the rock at 15 out and reaches the
    /// dps disk; lurer A, out for a monster 28 out, passes it and tags the monster. Neither
    /// body enters a rock.
    #[test]
    fn runners_round_a_rock_in_their_path_and_arrive() {
        // A rock in a monster's path lies outside the dps disk (inside it a monster fights
        // unhindered), the monster within the chase's line of sight of 35; one in the lurer's
        // path lies between the centre and its stop, 15 short of a monster it sees.
        let circle = |at: Vec2| Rocks {
            circles: vec![Circle::new(at, 3.0, SeenFrom::Outside)],
            ..Default::default()
        };
        let slab = |at: Vec2, angle: f32| Rocks {
            rectangles: vec![rectangle(at, 1.5, 4.0, angle)],
            ..Default::default()
        };
        let shapes: [&dyn Fn(Vec2) -> Rocks; 3] =
            [&circle, &|at| slab(at, 0.0), &|at| slab(at, 0.4)];
        for shape in shapes {
            // The monster.
            let rocks = shape(Vec2::new(-25.0, 0.0));
            let mut field = field_with_a_monster_at(Vec2::new(-34.0, 0.0));
            field.obstacles = rocks.clone();
            field.monsters[0].threat.insert(0, 1.0);
            let id = field.monsters[0].id;
            let (mut closest, mut nearest_in) = (f32::MAX, f32::MAX);
            for _ in 0..600 {
                field.update(1.0 / 60.0);
                field.team[0].body = standing(CENTRE);
                if let Some(m) = field.monsters.iter().find(|m| m.id == id) {
                    closest = closest.min(clearance(&rocks, &m.body));
                    nearest_in = nearest_in.min(m.body.position.length());
                }
            }
            assert!(closest >= -0.05, "{closest} {rocks:?}");
            assert!(nearest_in <= DPS_RADIUS, "{nearest_in} {rocks:?}");
            // The lurer.
            let rocks = shape(Vec2::new(-9.0, 0.0));
            let mut field = field_with_a_monster_at(Vec2::new(-30.0, 0.0));
            field.obstacles = rocks.clone();
            field.monsters[0].gait = Gait::Standing;
            field.monsters[0].until = f32::MAX;
            // Tagged, the monster follows the lurer into the disk and dies there within the
            // ten seconds, so the tag is tracked by id along the way.
            let id = field.monsters[0].id;
            let (mut closest, mut tagged) = (f32::MAX, false);
            for _ in 0..600 {
                field.update(1.0 / 60.0);
                closest = closest.min(clearance(&rocks, &field.team[0].body));
                tagged |= (field.monsters.iter()).any(|m| m.id == id && m.threat.contains_key(&0));
            }
            assert!(closest >= -0.05, "{closest} {rocks:?}");
            assert!(tagged, "{rocks:?}");
        }
    }

    /// A monster already chasing a player or already inside the dps disk is never a pull
    /// target; the same monster idle outside the disk is.
    #[test]
    fn a_monster_already_fighting_or_in_the_disk_is_never_pulled() {
        let both = [Some(CENTRE), Some(CENTRE)];
        let mut field = quiet_field();
        // On lurer A's side (west): one 25 out, one 15 out (inside the dps disk).
        field.monsters[0].body = standing(Vec2::new(-25.0, 0.0));
        field.monsters[1].body = standing(Vec2::new(-15.0, 0.0));
        let outside = field.monsters()[0].id;
        let ids = |c: Vec<(u32, Vec2)>| c.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        assert_eq!(
            ids(tour_candidates(0, CENTRE, field.monsters(), &both, true)),
            vec![outside]
        );
        assert_eq!(
            untagged_in_reach(0, Vec2::new(-12.0, 0.0), field.monsters(), &both, true),
            Some(0)
        );
        // The outside one chasing a dps: no candidate at all, nothing in reach.
        field.monsters[0].target = Some(2);
        assert!(tour_candidates(0, CENTRE, field.monsters(), &both, true).is_empty());
        assert_eq!(
            untagged_in_reach(0, Vec2::new(-12.0, 0.0), field.monsters(), &both, true),
            None
        );
        // Over five seconds lurer A never tags it (it scouts instead); idle, it is tagged.
        let chasing = |target: Option<usize>| {
            let mut field = quiet_field();
            field.monsters[0].body = standing(Vec2::new(-25.0, 0.0));
            field.monsters[0].spawn = field.monsters[0].body.position;
            field.monsters[0].target = target;
            if target.is_some() {
                field.monsters[0].threat.insert(2, 10.0);
            }
            let id = field.monsters()[0].id;
            run_tracking_a(&mut field, 300).contains(&id)
        };
        assert!(!chasing(Some(2)));
        assert!(chasing(None));
    }

    /// The default field drawn from `seed`: the monsters and rocks redrawn, everything else as
    /// `Default`.
    fn seeded(seed: u64) -> TrainingField {
        let mut field = TrainingField {
            rng: fastrand::Rng::with_seed(seed),
            ..TrainingField::default()
        };
        field.monsters.clear();
        field.respawns.clear();
        field.spawn_missing();
        field.obstacles = place_rocks(DEFAULT_OBSTACLES, &field.nests());
        field
    }

    /// Three minutes on each of seeds 1-5 (one seed misled the cap decision once): nobody
    /// dies, both lurers deliver, nothing is lost, and no lurer leaves the centre while a
    /// monster chases it.
    #[test]
    fn five_seeds_train_for_three_minutes_without_a_death() {
        // Measured 2026-11-03 (rocks, cap off, the hand-off): kills 36 / 41 / 22 / 31 / 36,
        // no deaths, the lowest member 42-48%, delivered A 24 / 29 / 13 / 20 / 25 and B 12 /
        // 17 / 15 / 11 / 18, nothing lost, no leave while chased. Before the hand-off: 5
        // deaths over seeds 2-4 and leaves while chased on most trips. With the healer's MP
        // gate and the bar-by-bar group heal: kills 36 / 41 / 22 / 30 / 36, B 8 on seed 4.
        // The lurer's knowledge (2026-11-05: HP from the damage seen, bars in tenths): kills
        // 36 / 44 / 25 / 22 / 36, delivered A 24 / 32 / 15 / 17 / 25 and B 12 / 17 / 18 / 8 /
        // 18, no deaths, nothing lost, no leave while chased, the lowest bar read 40 on every
        // seed (a tenth: somewhere in 40-50%), group heals 0 / 0 / 1 / 0 / 0. World units
        // (2026-11-05): kills 32 / 39 / 29 / 25 / 34 (159), delivered A 16 / 18 / 15 / 15 /
        // 21 and B 20 / 19 / 16 / 20 / 17, lost 0 / 1 / 0 / 1 / 0, no deaths, no leave while
        // chased, the lowest bar 40 / 80 / 50 / 40 / 50.
        let mut deaths = 0;
        for seed in 1..=5 {
            let mut field = seeded(seed);
            let mut last = [0u32; LURERS];
            let mut lowest = 100.0f32;
            for _ in 0..180 * 60 {
                field.update(1.0 / 60.0);
                lowest = lowest.min(field.lowest_hp());
                for (i, lure) in field.lures.iter().enumerate() {
                    if lure.trips > last[i] {
                        last[i] = lure.trips;
                        let chased = field.monsters.iter().any(|m| m.target == Some(i));
                        assert!(!chased, "seed {seed}: lurer {i} left while chased");
                    }
                }
            }
            let (a, b) = (&field.lures[0], &field.lures[1]);
            assert!(field.kills >= 15, "seed {seed}: {} kills", field.kills);
            assert!(lowest >= 30.0, "seed {seed}: lowest {lowest}");
            assert!(
                a.delivered >= 6 && b.delivered >= 6,
                "seed {seed}: {} {}",
                a.delivered,
                b.delivered
            );
            assert!(
                a.lost + b.lost <= 2,
                "seed {seed}: {} {} lost",
                a.lost,
                b.lost
            );
            deaths += field.deaths;
        }
        assert!(deaths <= 1, "{deaths} deaths over five seeds");
    }

    /// Two minutes of the default field: the lurer makes trips, brings monsters in, and is
    /// out of the dps disk most of the time.
    #[test]
    fn trips_deliver_monsters_to_the_dps_disk() {
        let mut field = TrainingField::default();
        let mut stages = std::collections::HashSet::new();
        for _ in 0..120 * 60 {
            field.update(1.0 / 60.0);
            stages.insert(field.lures[0].stage);
        }
        let (a, b) = (&field.lures[0], &field.lures[1]);
        // Measured 2026-11-03 with two lurers and four dps: A 3 trips, 10 delivered, 0 lost,
        // 3 sessions of 16.1 s, back with 100 HP, 18 walked per delivered, 73% inside; B 4
        // trips, 16 delivered, 1 lost, 4 sessions of 8.4 s, back with 98, 6 walked per
        // delivered, 93% inside (the load cap holds both while the dps clear); 26 kills, no
        // deaths. With 15 monsters (the user's default): A 2 trips, 5 delivered, 2 lost, 2
        // sessions of 20.6 s, 74% inside; B 3 trips, 9 delivered, 2 lost, 3 sessions of
        // 11.8 s, 87% inside; 16 kills, 5 deaths, all dps, to a giant and a party champion the
        // seed put near the disk (2,000 HP against four dps spreading fire, and dps roaming
        // past the HP buffer's reach). Load cap 6 at 15 monsters: 13 kills, 6 deaths; kept at 3.
        // With the heal, group heal and guard: the same trips and deliveries (A 2/5/2, B
        // 3/9/2), 16 kills, no deaths; 9 heals, 0 group heals, 10 guards. With the splash
        // rule (a packmate a dps's damage takes over within the blast's 2 outside the disk
        // counts delivered): A 2/7/0, B 3/11/0, 23 / 11 walked per delivered, the same 16
        // kills, no deaths, 9 heals, 10 guards; every former "lost" was a splashed packmate.
        // Strong cap 2 (from 1): the same two minutes (the giant and the party champion
        // arrive together at 50 s and two strong hold both lurers until 120.5 s either way);
        // at 180 s A 5/15/0, B 5/17/0, 24 kills, no deaths, at the centre 93 s / 113 s of
        // 180 (cap 1: A 3 and B 4 trips, 23 kills, 118 s / 123 s; a lone party normal held
        // both from 138 s on). With 8 rocks (the default): A 4/10/0, B 4/14/0, 4 sessions of
        // 17.0 s / 11.3 s, 27 / 11 walked per delivered, 61% / 83% inside, 22 kills, no
        // deaths, 1 heal, 18 guards; the rocks move every path, so the giant arrives at 40 s
        // and two party champions by 80 s (each lurer planned its strong one while the disk
        // held one), 6,000 HP that hold both lurers from 76 s past 180 s (at the centre 112 s
        // / 135 s; the third-minute bound holds without margin: 4 trips each).
        // Rocks as circles and rectangles (a coin per rock; the layout redrawn): none of
        // this layout's 8 lies on the first two minutes' paths, so the two minutes are the
        // no-rock numbers again; at 180 s A 5/15/0, B 5/18/0, 28 kills, no deaths, at the
        // centre 93 s / 94 s; closest clearance to a rock 0.00. The risk check from the
        // lowest bar (70 / 15 and 50 / 30 instead of the mean's 80 / 10 and 60 / 20): A 3
        // trips, 7 delivered, 2 sessions of 21.5 s, 33 walked per delivered, 62% inside; B
        // 4/15/0, 4 sessions of 10.3 s, 9 walked, 88%; 17 kills, no deaths, 2 heals, 15
        // guards; at 180 s A 3/11/0, B 4/15/0, 21 kills, at the centre 114 s / 139 s (the
        // halts at a dps under 50% at 50, 80 and 120 s that the mean never saw).
        // The backlog cap (30 s of dps work, in place of the load cap 3 and the strong cap
        // 2): A 3/9/0, 3 sessions of 21.1 s, 28 walked per delivered, 60% inside; B 12/16/0,
        // 12 sessions of 4.9 s (short pulls whenever the work runs low), 13 walked, 73%; 17
        // kills, no deaths, 1 heal, 13 guards; at 180 s A 5/14/0, B 14/20/0, 25 kills, at the
        // centre 89 s / 107 s. The pull rule (nothing chasing or inside the disk is pulled)
        // and the cap off by default (the cap tables in PLAN.md: no cap ever bought safety,
        // every cap cost kills): A 44/16/0 in 2.7 s pulls (the near ring's roamers), 28
        // walked per delivered, 32% inside; B 10/21/0, 9.6 s, 15 walked, 71%; 33 kills, no
        // deaths, 1 heal, 20 guards; at 180 s A 55/25/0, B 13/26/0, 44 kills, at the centre
        // 2 s / 35 s, lowest member 48%. The hand-off (no leave while anything chases the
        // lurer): A 6/19/0 in 18.7 s trips, 21 walked per delivered, 40% inside; B 4/11/0,
        // 11.2 s, 14 walked, 81%; 23 kills, no deaths, 1 heal, 18 guards; at 180 s A 9/24,
        // B 6/12, 36 kills. Over seeds 1-5 (180 s, cap off): 166 kills and no deaths against
        // 190 kills and 5 deaths without the hand-off, which had lurers leaving with their
        // pack still behind them on 54 of 68 leaves (seed 1) and dragging strong monsters
        // back out; cap 30 s with the hand-off: 105 kills, no deaths. The healer's MP gate
        // (no strong pull under 50 MP): the same numbers on all five seeds (the healer fell
        // to 20 MP on seed 4 and 53-73 elsewhere, never while a strong pull was on offer).
        // The group heal on two members under 70% (on the party's mean it never fired): seed
        // 1 unchanged; over the five seeds 165 kills, no deaths, group heals 0 / 0 / 1 / 4 / 0
        // (seed 4's four cost the healer all its MP, so its strong pulls paused: B 8
        // delivered instead of 11). The lurer's knowledge from the watch log (a monster's HP
        // as max HP less the damage the lurer saw; the party window in tenths): seed 1
        // unchanged at 120 and 180 s; over the five seeds 163 kills, no deaths (the tenths
        // moved seeds 2-4), and at 180 s the lurers' known backlog overshoots the true one by
        // the damage they missed (seed 3: 168 / 192 s known against 157). World units
        // (2026-11-05, the dps disk 20, the luring radius 40, run 11, monsters 10 / 4, the
        // reaches 15 / 8 / 4, the dps blast 5): A 9/13/0 in 7.3 s sessions, 26 walked per
        // delivered, 88% inside the (now 20) disk; B 13/17/0, 7.3 s, 28 walked, 80%; 29
        // kills, no deaths, 1 heal, 23 guards; at 180 s A 10/16/0, B 14/20/0, 32 kills. With
        // the blast left at 2 the kills stopped at 13 from 80 s on: biters stand 4 from the
        // lurer, up to 8 apart, and a blast of 2 caught one at a time while the dps drained
        // their MP (the sweep in PLAN.md Findings).
        assert_eq!(stages.len(), 3);
        let delivered = a.delivered + b.delivered;
        assert!(a.trips >= 2 && b.trips >= 2, "{} {}", a.trips, b.trips);
        assert!(delivered >= 10, "{delivered}");
        assert!(a.lost + b.lost <= 1, "{} {} lost", a.lost, b.lost);
        assert!(
            a.sessions >= 2 && b.sessions >= 2,
            "{} {}",
            a.sessions,
            b.sessions
        );
        assert!(a.mean_hp_back() > 60.0 && b.mean_hp_back() > 60.0);
        assert!(
            field.kills >= 8 && field.deaths <= 2,
            "{} {}",
            field.kills,
            field.deaths
        );
        // The third minute: one strong monster in the disk no longer holds the lurers (4
        // trips each at strong cap 2; 3 and 4 since the lowest-bar risk check; 55 and 13
        // with the cap off).
        for _ in 0..60 * 60 {
            field.update(1.0 / 60.0);
        }
        let (a, b) = (&field.lures[0], &field.lures[1]);
        assert!(a.trips >= 3 && b.trips >= 3, "{} {}", a.trips, b.trips);
        assert!(field.deaths <= 2, "{}", field.deaths);
    }

    #[test]
    fn the_lurer_tags_each_monster_in_reach_once_a_second_and_never_twice() {
        let mut field = quiet_field();
        // Two monsters 10 from the parked lurer on opposite sides (no blast or pierce catches
        // both; beyond their alarm of 8, so neither chases first), away from the team; the
        // first first.
        let den = Vec2::new(60.0, 40.0);
        for (k, m) in field.monsters.iter_mut().take(2).enumerate() {
            m.body = standing(den + Vec2::new(0.0, if k == 0 { 10.0 } else { -10.0 }));
            m.spawn = m.body.position;
            // Standing, so the second stays in reach for its turn.
            m.gait = Gait::Standing;
            m.until = f32::MAX;
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
        field.team[2].hp = 50.0;
        field.team[3].mp = 50.0;
        field.update(1.0 / 60.0);
        let t = field.time;
        assert!((field.team()[2].hp_buff_until - (t + BUFF_DURATION)).abs() < 1e-3);
        assert!((field.team()[3].mp_buff_until - (t + BUFF_DURATION)).abs() < 1e-3);
        assert!((field.team()[6].mp - (MAX_MP - HP_BUFF_MP)).abs() < 0.1);
        assert!((field.team()[7].mp - (MAX_MP - MP_BUFF_MP)).abs() < 0.1);
        // 5 s in: 50 + 5 x (4 + 1) HP, 50 + 5 x (3 + 1) MP.
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!(
            (field.team()[2].hp - 75.0).abs() < 0.5,
            "{}",
            field.team()[2].hp
        );
        assert!(
            (field.team()[3].mp - 70.0).abs() < 0.5,
            "{}",
            field.team()[3].mp
        );
        // No second cast while the buff runs: the buffers' MP only regenerated.
        assert!(field.team()[6].mp > MAX_MP - HP_BUFF_MP);
        assert!(field.team()[7].mp > MAX_MP - MP_BUFF_MP);
        // At 10 s the HP buff has given 40 and regeneration 10: full. The MP buff has given
        // 30 and regeneration 10: 90, the threshold, so no fresh cast follows.
        for _ in 0..300 {
            field.update(1.0 / 60.0);
        }
        assert!(field.team()[2].hp >= 99.0, "{}", field.team()[2].hp);
        assert!(
            (field.team()[3].mp - 90.0).abs() < 0.5,
            "{}",
            field.team()[3].mp
        );
    }

    #[test]
    fn a_buff_needs_a_bar_under_the_threshold_within_reach_and_goes_to_the_lowest_first() {
        let mut field = quiet_field();
        // 95%: above the threshold, nothing cast.
        field.team[2].hp = 95.0;
        field.update(1.0 / 60.0);
        assert_eq!(field.team()[2].hp_buff_until, 0.0);
        // The lurer at 50% but 40 away: out of the HP buffer's reach.
        let mut field = quiet_field();
        field.team[0].hp = 50.0;
        field.team[0].body = standing(Vec2::new(40.0, 0.0));
        field.update(1.0 / 60.0);
        assert_eq!(field.team()[0].hp_buff_until, 0.0);
        // Three in need (all over 50%, so no heal): the lowest first (the buffer buffs
        // itself), then the others in order of need, one cast a second, each when in reach of
        // the roaming buffer.
        let mut field = quiet_field();
        field.team[2].hp = 85.0;
        field.team[3].hp = 75.0;
        field.team[6].hp = 65.0;
        let mut buffed_at = [f32::MAX; 8];
        for _ in 0..600 {
            field.update(1.0 / 60.0);
            for (i, p) in field.team().iter().enumerate() {
                if p.hp_buff_until > 0.0 {
                    buffed_at[i] = buffed_at[i].min(field.time);
                }
            }
        }
        assert!(buffed_at[6] < 0.1, "{buffed_at:?}");
        assert!(
            buffed_at[6] < buffed_at[3] && buffed_at[3] < buffed_at[2],
            "{buffed_at:?}"
        );
        assert!(buffed_at[2] < 10.0, "{buffed_at:?}");
        // Nobody else was buffed: the lurers, the other two dps, the MP buffer.
        assert!([0, 1, 4, 5, 7].iter().all(|&i| buffed_at[i] == f32::MAX));
    }

    #[test]
    fn the_four_dps_fire_at_a_monster_in_the_disk_and_feed_its_threat() {
        // 4 below the centre: within 8 of all three dps (5.7, 7.7 and 2.1 away).
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        field.update(1.0 / 60.0);
        let m = &field.monsters()[0];
        // Four shots; no tag from lurer A, which leaves a monster inside the disk alone.
        assert_eq!(m.hp, MAX_HP - 4.0 * AOE_DAMAGE);
        let threat = HashMap::from([(2, 10.0), (3, 10.0), (4, 10.0), (5, 10.0)]);
        assert_eq!(m.threat, threat);
        assert_eq!(m.target, Some(2));
        for (i, p) in field.team().iter().enumerate() {
            let spent = match p.role {
                Role::Dps => AOE_MP,
                _ => 0.0,
            };
            assert_eq!(p.target, (spent > 0.0).then_some(0), "player {i}");
            assert!((p.mp - (MAX_MP - spent)).abs() < 0.1, "player {i}");
        }
        // Volleys at 0, 1.5 and 3 s: dead at the third, replaced 3 s later.
        for _ in 0..(5.0 * 60.0) as usize {
            field.update(1.0 / 60.0);
        }
        assert_eq!(field.kills, 1);
        assert_eq!(field.monsters().len(), 4);
    }

    #[test]
    fn the_blast_hurts_every_monster_within_five_of_the_target_only() {
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        field.monsters[1].body = standing(Vec2::new(0.0, -8.0));
        field.monsters[2].body = standing(Vec2::new(0.0, -10.0));
        field.update(1.0 / 60.0);
        let hp: Vec<f32> = field.monsters().iter().map(|m| m.hp).collect();
        // Four blasts on the first (the second within 5 of it, the third 6 away); lurer A
        // leaves monsters inside the disk alone.
        assert_eq!(hp[..3], [60.0, 60.0, 100.0]);
    }

    #[test]
    fn a_dps_waits_for_mp_and_runs_into_reach_first() {
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -4.0));
        for p in &mut field.team {
            p.mp = AOE_MP - 1.0;
        }
        field.update(1.0 / 60.0);
        // No shot, and no tag either: the lurer leaves a monster inside the disk alone.
        assert_eq!(field.monsters()[0].hp, MAX_HP);
        for _ in 0..70 {
            field.update(1.0 / 60.0);
        }
        assert!(field.monsters()[0].hp <= MAX_HP - AOE_DAMAGE);
        // A monster inside the disk but out of reach of the dps at (2.8, 2.8): 22 away.
        let mut field = field_with_a_monster_at(Vec2::new(0.0, -19.0));
        field.update(1.0 / 60.0);
        let dps = &field.team()[2];
        assert_eq!(dps.target, Some(0));
        assert!(dps.body.speed > 0.0);
        for _ in 0..120 {
            field.update(1.0 / 60.0);
        }
        let dps = &field.team()[2];
        let gap = (field.monsters()[0].body.position - dps.body.position).length();
        assert!(gap <= RANGED_REACH + 0.5, "{gap}");
        assert!(field.monsters()[0].hp <= MAX_HP - AOE_DAMAGE);
    }

    #[test]
    fn the_team_is_eight_roles_standing_inside_their_disks() {
        let field = TrainingField::default();
        let roles: Vec<Role> = field.team().iter().map(|p| p.role).collect();
        assert_eq!(
            roles,
            [
                Role::Lurer,
                Role::Lurer,
                Role::Dps,
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
            assert!(off.length() <= MAP_RADIUS - IDLE_RADIUS);
            assert!(off.length() > DPS_RADIUS);
            assert_eq!(m.spawn, m.body.position);
        }
        // They spread: some in the lurer's disk, some beyond it.
        let in_reach = field
            .monsters()
            .iter()
            .filter(|m| (m.body.position - CENTRE).length() <= LURE_RADIUS)
            .count();
        assert!(in_reach > 2 && in_reach < DEFAULT_MONSTERS, "{in_reach}");
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
        // At a run of 11 the roam's braking leaves a body up to half a radius past its edge.
        assert!(m.player_over <= 0.5, "{}", m.player_over);
        assert!(
            m.mean_speed[2..6].iter().all(|&s| s > 2.3),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.mean_speed[6..].iter().all(|&s| s > 1.4),
            "{:?}",
            m.mean_speed
        );
        assert!(
            m.min_speed[2..].iter().all(|&s| s > 0.7),
            "{:?}",
            m.min_speed
        );
        // The dps roam a disk of 20 and the HP buffer reaches 15 from within 5, so the dps
        // need not cross its disk (they did 42-49% of the time when the disks were 10 and
        // 5); the MP buffer roams like a dps.
        assert!(
            m.in_buff_disk[2..6].iter().all(|&s| s > 0.03),
            "{:?}",
            m.in_buff_disk
        );
        assert!(m.in_buff_disk[6] > 0.99, "{:?}", m.in_buff_disk);
        assert!(m.in_buff_disk[7] > 0.03, "{:?}", m.in_buff_disk);
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
        // Never chased at seed 1: three before the lowest-bar risk check, two after it, one
        // with the backlog cap off (the lurers pull far more).
        assert!(!quiet.is_empty(), "{}", quiet.len());
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
