"""Parse Caravan's bot.watch lines and rebuild entity tracks (Traps rules of watch-log.md)."""

import ast
import math
import re
from dataclasses import dataclass, field
from datetime import datetime

LINE = re.compile(r'(\S+)\s+\w+\s+\S+: (\w+) event="bot.watch" account=\S+ (.*)')
FIELD = re.compile(r'(\w+)=("[^"]*"|\[.*?\](?= \w+=| *$)|\S+)')

MOTION_WALK, MOTION_RUN, MOTION_SIT = 2, 3, 4


def value(raw):
    if raw.startswith('"'):
        return raw[1:-1]
    if raw.startswith('['):
        return ast.literal_eval(raw.replace("false", "False").replace("true", "True"))
    if raw in ("true", "false"):
        return raw == "true"
    try:
        return int(raw)
    except ValueError:
        pass
    try:
        return float(raw)
    except ValueError:
        return raw


def parse(path):
    """Yield (t_seconds_since_first_line, ev, fields) per bot.watch line."""
    t0 = None
    with open(path) as f:
        for line in f:
            m = LINE.match(line)
            if not m:
                continue
            stamp, ev, rest = m.groups()
            t = datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()
            if t0 is None:
                t0 = t
            fields = {k: value(v) for k, v in FIELD.findall(rest)}
            yield t - t0, ev, fields


def world(region, x, y):
    return ((region & 0xFF) - 135) * 192 + x / 10, ((region >> 8) - 92) * 192 + y / 10


@dataclass
class Leg:
    t0: float
    x0: float
    y0: float
    x1: float
    y1: float
    speed: float

    def duration(self):
        d = math.hypot(self.x1 - self.x0, self.y1 - self.y0)
        return d / self.speed if self.speed > 0 else math.inf

    def at(self, t):
        dur = self.duration()
        if dur == 0 or t >= self.t0 + dur:
            return self.x1, self.y1
        f = max(0.0, (t - self.t0) / dur)
        return self.x0 + f * (self.x1 - self.x0), self.y0 + f * (self.y1 - self.y0)


@dataclass
class Entity:
    uid: int
    kind: str
    ref_id: int
    name: str
    rarity: int
    max_hp: int
    walk: float
    run: float
    motion: int = MOTION_RUN
    x: float = 0.0
    y: float = 0.0
    leg: Leg | None = None
    segments: list = field(default_factory=list)  # (t0, t1, x0, y0, x1, y1, speed, motion)
    first_seen: float = 0.0
    last_seen: float = 0.0
    dead_at: float | None = None

    def speed(self):
        if self.motion == MOTION_WALK:
            return self.walk
        if self.motion == MOTION_SIT:
            return 0.0
        return self.run

    def pos_at(self, t):
        if self.leg and t >= self.leg.t0:
            return self.leg.at(t)
        return self.x, self.y

    def settle(self, t):
        """End the current leg at t: the position becomes where the leg had it."""
        if self.leg:
            x, y = self.leg.at(t)
            t1 = min(t, self.leg.t0 + self.leg.duration())
            self.segments.append((self.leg.t0, t1, self.leg.x0, self.leg.y0, x, y, self.leg.speed, self.motion))
            self.x, self.y, self.leg = x, y, None

    def start_leg(self, t, x1, y1, from_xy=None):
        self.settle(t)
        if from_xy is not None:
            self.x, self.y = from_xy
        self.leg = Leg(t, self.x, self.y, x1, y1, self.speed())

    def place(self, t, x, y):
        self.settle(t)
        self.x, self.y = x, y


class World:
    """Replays the records; keeps entities and the tables the analysis wants."""

    def __init__(self):
        self.entities = {}
        self.gone = []  # entities that despawned (kept for the tables)
        self.casts = []
        self.hits = []
        self.buff_on = {}  # token -> (t, target, skill)
        self.buffs = []  # (target, skill, token, t_on, t_off)
        self.deaths = []
        self.me = []
        self.npcs = {}
        self.skills = {}
        self.session = None
        self.names = {}  # uid -> name (players)
        self.index_errors = 0
        self.last_i = None
        self.actions = {}  # action id -> (caster, skill) from its cast

    def find(self, uid):
        return self.entities.get(uid)

    def apply(self, t, ev, f):
        if "i" in f:
            if self.last_i is not None and f["i"] < self.last_i:
                self.index_errors += 1
            self.last_i = f["i"]
        getattr(self, "on_" + ev, self.on_other)(t, f)

    def on_other(self, t, f):
        pass

    def on_session(self, t, f):
        self.session = f

    def on_def_npc(self, t, f):
        self.npcs[f["ref_id"]] = f

    def on_def_skill(self, t, f):
        self.skills[f["skill"]] = f

    def on_spawn(self, t, f):
        e = Entity(
            uid=f["uid"], kind=f["kind"], ref_id=f["ref_id"], name=f.get("name", ""),
            rarity=f.get("rarity", 0), max_hp=f.get("max_hp", 0),
            walk=f.get("walk", 0.0), run=f.get("run", 0.0),
            x=f["wx"], y=f["wy"], first_seen=t, last_seen=t,
        )
        if f.get("life") == 2:
            e.dead_at = t
        e.spawn_group = f.get("group", False)
        self.entities[f["uid"]] = e
        if e.kind == "Player":
            self.names[e.uid] = e.name
        if "dest_x" in f:
            x1, y1 = world(f["dest_region"], f["dest_x"], f["dest_y"])
            e.start_leg(t, x1, y1)

    def on_despawn(self, t, f):
        e = self.entities.pop(f["uid"], None)
        if e:
            e.settle(t)
            e.last_seen = t
            e.despawn_reason = f.get("reason")
            self.gone.append(e)

    def on_walk(self, t, f):
        e = self.find(f["uid"])
        if not e:
            return
        e.last_seen = t
        if "wx" not in f:
            return
        from_xy = (f["from_wx"], f["from_wy"]) if "from_wx" in f else None
        e.start_leg(t, f["wx"], f["wy"], from_xy)

    def on_pos(self, t, f):
        e = self.find(f["uid"])
        if e:
            e.place(t, f["wx"], f["wy"])
            e.last_seen = t

    def on_speed(self, t, f):
        e = self.find(f["uid"])
        if not e:
            return
        e.walk, e.run = f["walk"], f["run"]
        self.retime(e, t)

    def on_state(self, t, f):
        e = self.find(f["uid"])
        if not e:
            return
        if f.get("axis") == "motion":
            e.motion = f["value"]
            self.retime(e, t)

    def retime(self, e, t):
        if e.leg and t < e.leg.t0 + e.leg.duration():
            x1, y1 = e.leg.x1, e.leg.y1
            e.start_leg(t, x1, y1)

    def on_cast(self, t, f):
        c = dict(f)
        c["t"] = t
        caster = self.find(f["caster"])
        c["caster_name"] = caster.name if caster else self.names.get(f["caster"], "")
        c["caster_kind"] = caster.kind if caster else f.get("kind", "")
        c["caster_ref"] = caster.ref_id if caster else f.get("ref_id")
        # Our track's positions before the landing is placed (section 7 check 1).
        c["my_caster_xy"] = caster.pos_at(t) if caster else None
        target = self.find(f["target"])
        c["my_target_xy"] = target.pos_at(t) if target else None
        c["target_kind"] = target.kind if target else ""
        c["target_name"] = target.name if target else ""
        c["target_ref"] = target.ref_id if target else None
        self.casts.append(c)
        self.actions[f["action"]] = (f["caster"], f["skill"])
        for h in f.get("hits", []):
            uid, damage, crit, blocked, dead = h
            v = self.find(uid)
            self.hits.append(dict(t=t, cast_index=len(self.casts) - 1, caster=f["caster"], skill=f["skill"],
                                  uid=uid, damage=damage, crit=crit, blocked=blocked, dead=dead,
                                  victim_kind=v.kind if v else "", victim_ref=v.ref_id if v else None,
                                  victim_name=v.name if v else ""))
        for kb in f.get("knockback", []):
            uid, region, x, y, z = kb
            v = self.find(uid)
            if v:
                v.place(t, *world(region, x, y))
        tp = f.get("teleport")
        if tp and caster:
            region, x, y, z = ast.literal_eval(tp)
            caster.place(t, *world(region, x, y))

    def on_cast_end(self, t, f):
        caster, skill = self.actions.get(f["action"], (None, None))
        for h in f.get("hits", []):
            uid, damage, crit, blocked, dead = h
            v = self.find(uid)
            self.hits.append(dict(t=t, cast_index=None, caster=caster, skill=skill, action=f["action"],
                                  uid=uid, damage=damage, crit=crit, blocked=blocked, dead=dead,
                                  victim_kind=v.kind if v else "", victim_ref=v.ref_id if v else None,
                                  victim_name=v.name if v else ""))

    def on_buff_on(self, t, f):
        self.buff_on[f["token"]] = (t, f["target"], f["skill"])

    def on_buff_off(self, t, f):
        on = self.buff_on.pop(f["token"], None)
        if on:
            t_on, target, skill = on
            self.buffs.append(dict(target=target, skill=skill, token=f["token"], t_on=t_on, t_off=t, duration=t - t_on))
        else:
            self.buffs.append(dict(target=f.get("target"), skill=f.get("skill"), token=f["token"], t_on=None, t_off=t, duration=None))

    def on_life(self, t, f):
        e = self.find(f["uid"])
        if f["value"] == 2:
            if e:
                e.dead_at = t
            self.deaths.append(dict(t=t, uid=f["uid"], kind=f.get("kind"), ref_id=f.get("ref_id"),
                                    last_caster=f.get("last_caster"), last_skill=f.get("last_skill"),
                                    name=e.name if e else ""))
        elif e:
            e.dead_at = None

    def on_me(self, t, f):
        self.me.append(dict(t=t, hp=f.get("hp"), mp=f.get("mp"), attacked_by=f.get("attacked_by", []),
                            untouchable=f.get("untouchable"), dead=f.get("dead")))

    def finish(self, t):
        for e in self.entities.values():
            e.settle(t)
            e.last_seen = max(e.last_seen, t)

    def all_entities(self):
        return list(self.gone) + list(self.entities.values())


def load(path):
    w = World()
    t = 0.0
    for t, ev, f in parse(path):
        w.apply(t, ev, f)
    w.finish(t)
    w.end = t
    return w


def pos_from_segments(segments, t, fallback):
    """Position at t from a finished entity's segments (binary search would do; lists are short)."""
    x, y = fallback
    for (t0, t1, x0, y0, x1, y1, speed, motion) in segments:
        if t < t0:
            return x0, y0
        if t <= t1:
            d = t1 - t0
            f = (t - t0) / d if d > 0 else 1.0
            return x0 + f * (x1 - x0), y0 + f * (y1 - y0)
        x, y = x1, y1
    return x, y


@dataclass
class Merged:
    """Every record of one uid, joined: an entity that left sight and came back."""
    uid: int
    kind: str
    ref_id: int
    name: str
    rarity: int
    max_hp: int
    walk: float
    run: float
    segments: list
    intervals: list  # (first_seen, last_seen) per record
    first_group: bool
    dead_at: float | None
    x: float
    y: float

    @property
    def first_seen(self):
        return self.intervals[0][0]

    @property
    def last_seen(self):
        return self.intervals[-1][1]

    def seen(self, t):
        if self.dead_at is not None and t >= self.dead_at:
            return False
        return any(a <= t <= b for a, b in self.intervals)

    def pos_at(self, t):
        return pos_from_segments(self.segments, t, (self.x, self.y))


def merge(w):
    by_uid = {}
    for e in sorted(w.all_entities(), key=lambda e: e.first_seen):
        m = by_uid.get(e.uid)
        if m is None:
            by_uid[e.uid] = Merged(e.uid, e.kind, e.ref_id, e.name, e.rarity, e.max_hp, e.walk, e.run,
                                   list(e.segments), [(e.first_seen, e.last_seen)], getattr(e, "spawn_group", False),
                                   e.dead_at, e.x, e.y)
        else:
            m.segments.extend(e.segments)
            m.intervals.append((e.first_seen, e.last_seen))
            m.dead_at = e.dead_at
            m.x, m.y = e.x, e.y
            m.walk, m.run, m.rarity, m.max_hp = e.walk, e.run, e.rarity, e.max_hp
    for m in by_uid.values():
        m.segments.sort(key=lambda s: s[0])
    return by_uid


def slice_log(path, account, start, end):
    """Yield the bot.watch lines of `account` whose timestamp is in [start, end) (ISO prefixes).

    Reads the whole file once (about 300 MB, 10-20 s); write the result to a file outside git
    when it is needed more than once (it holds other players' names).
    """
    needle = f'event="bot.watch" account={account} '
    with open(path, errors="replace") as f:
        for line in f:
            if needle not in line:
                continue
            stamp = line[: line.find(" ")]
            if stamp < start:
                continue
            if stamp >= end:
                break
            yield line


def common_args(parser):
    """The arguments every script takes: a sliced file, or the log with a window."""
    parser.add_argument("--slice", help="a file of bot.watch lines already cut to the window")
    parser.add_argument("--log", default="~/.caravan/caravand.log", help="caravand.log (default ~/.caravan/caravand.log)")
    parser.add_argument("--account", help="the observer's account (with --log)")
    parser.add_argument("--from", dest="start", help="ISO start, e.g. 2026-11-05T18:15:28 (with --log)")
    parser.add_argument("--to", dest="end", help="ISO end, exclusive (with --log)")


def load_args(args):
    """A World from --slice, or from --log cut to [--from, --to)."""
    import os
    import tempfile

    if args.slice:
        return load(args.slice)
    if not (args.account and args.start and args.end):
        raise SystemExit("give --slice, or --log with --account, --from and --to")
    tmp = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
    with tmp:
        for line in slice_log(os.path.expanduser(args.log), args.account, args.start, args.end):
            tmp.write(line)
    try:
        return load(tmp.name)
    finally:
        os.unlink(tmp.name)
