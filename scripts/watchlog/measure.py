"""Section 5 of CARAVAN_LOGGER_REQUIREMENTS.md: the measurements over a watch log window (setup A)."""
import math
import statistics as st
import argparse
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import watchlog

parser = argparse.ArgumentParser(description='Section 5 of CARAVAN_LOGGER_REQUIREMENTS.md: the measurements over a watch log window (setup A).')
watchlog.common_args(parser)
args = parser.parse_args()
w = watchlog.load_args(args)
OBS = (w.session["wx"], w.session["wy"])
END = w.end
ents = watchlog.merge(w)
roles = dict(part.split("=") for part in w.session["roles"].split(","))
uid_by_name = defaultdict(set)
for e in ents.values():
    if e.kind == "Player":
        uid_by_name[e.name].add(e.uid)
party = {}
for name, role in roles.items():
    for uid in uid_by_name.get(name, ()):
        party[uid] = (name, role)
print("party uids:", {n: sorted(uid_by_name[n]) for n in roles})
LURERS = [u for u, (n, r) in party.items() if r == "lurer"]
DPS = [u for u, (n, r) in party.items() if r == "dps"]
BUFFERS = [u for u, (n, r) in party.items() if "buffer" in r]
players = [e for e in ents.values() if e.kind == "Player"]
monsters = [e for e in ents.values() if e.kind == "Monster"]
# The two commonest monster models: the dps damage is reported on their normal-rarity bodies.
TOP_MODELS = [ref for ref, _ in Counter(m.ref_id for m in monsters).most_common(2)]
TOP_HP = w.npcs.get(TOP_MODELS[0], {}).get("max_hp", 0) if TOP_MODELS else 0


def pos(uid, t):
    e = ents[uid]
    return watchlog.pos_from_segments(e.segments, t, (e.x, e.y))


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def pct(v, p):
    if not v:
        return None
    s = sorted(v)
    k = (len(s) - 1) * p / 100
    f, c = math.floor(k), math.ceil(k)
    return s[f] if f == c else s[f] + (s[c] - s[f]) * (k - f)


def q(v, digits=1):
    if not v:
        return "none"
    return f"n={len(v)} p10={pct(v,10):.{digits}f} p50={pct(v,50):.{digits}f} p90={pct(v,90):.{digits}f} max={max(v):.{digits}f}"


def seen(e, t):
    return e.seen(t)


hits_by_action = defaultdict(list)
for h in w.hits:
    if h.get("action") is not None:
        hits_by_action[h["action"]].append(h)


def cast_hits(c):
    """A cast's hits: on the cast itself, else on its cast_end (a cast-time skill)."""
    own = [(h[0], h[1]) for h in c.get("hits", [])]
    if own:
        return own
    return [(h["uid"], h["damage"]) for h in hits_by_action.get(c["action"], [])]


# Attacked stretches (R10): cut from every time-sampled measure.
attacked = []
start = None
for m in w.me:
    if m["attacked_by"] and start is None:
        start = m["t"]
    elif not m["attacked_by"] and start is not None:
        attacked.append((start, m["t"]))
        start = None
if start is not None:
    attacked.append((start, END))


def clean(t):
    return not any(a <= t < b for a, b in attacked)


print("\n== A. The centre and the disks (world units; the observer is the origin of nothing, the centre is the dps' median)")
samples = defaultdict(list)
for t in range(0, int(END)):
    if not clean(t):
        continue
    for uid, (name, role) in party.items():
        if seen(ents[uid], t):
            samples[uid].append(pos(uid, t))
dps_pts = [p for u in DPS for p in samples[u]]
centre = (st.median(p[0] for p in dps_pts), st.median(p[1] for p in dps_pts))
print(f"centre {centre[0]:.1f}, {centre[1]:.1f}; observer {dist(OBS, centre):.1f} from it")
for role_name, uids in (("dps", DPS), ("buffers", BUFFERS), ("lurers", LURERS)):
    d = [dist(p, centre) for u in uids for p in samples[u]]
    print(f"  {role_name} distance from the centre: {q(d)}")
for u in LURERS + DPS + BUFFERS:
    d = [dist(p, centre) for p in samples[u]]
    print(f"    {party[u][0]:12s} {party[u][1]:10s} {q(d)}")
death_d = [dist(pos(d['uid'], d['t']), centre) for d in w.deaths if d["kind"] == "Monster"]
print(f"  monsters' death points from the centre (all parties): {q(death_d)}")
party_kill_d = [dist(pos(d['uid'], d['t']), centre) for d in w.deaths if d["kind"] == "Monster" and d.get("last_caster") in party]
print(f"  deaths whose last hit was the party's: {q(party_kill_d)}")

print("\n== B. Player speeds (world units a second)")
for u in LURERS + DPS + BUFFERS:
    e = ents[u]
    runs = sorted({round(s[6], 2) for s in e.segments if s[7] == watchlog.MOTION_RUN and s[6] > 0})
    legs = [s for s in e.segments if s[7] == watchlog.MOTION_RUN and s[6] > 0 and s[1] > s[0]]
    tl = sum(s[1] - s[0] for s in legs)
    wavg = sum(s[6] * (s[1] - s[0]) for s in legs) / tl if tl else 0
    print(f"  {e.name:12s} {party[u][1]:10s} spawn run {e.run:.2f} walk {e.walk:.2f}; run speeds seen {runs}; time-weighted leg speed {wavg:.2f} over {tl:.0f} s")
bare = Counter(round(e.run, 2) for e in players)
print(f"  all players' current run speed (last seen): {bare.most_common(6)}")

print("\n== C. Monster speeds per model")
for ref, row in sorted(w.npcs.items()):
    ms = [e for e in monsters if e.ref_id == ref]
    if not ms:
        continue
    runs = Counter(round(e.run, 2) for e in ms)
    walks = Counter(round(e.walk, 2) for e in ms)
    print(f"  {ref} {row.get('name')!s:28s} level {row.get('level')} max_hp {row.get('max_hp')} gamedata walk/run wire {row.get('walk')}/{row.get('run')}; seen run {runs.most_common(3)} walk {walks.most_common(2)}; n={len(ms)}")
motion_by_leg = Counter()
for e in monsters:
    for s in e.segments:
        if s[1] > s[0] and s[6] > 0:
            motion_by_leg[s[7]] += 1
print(f"  monster legs by motion (2 walk, 3 run): {dict(motion_by_leg)}")

print("\n== D. Idle monsters: legs with no hit yet and no player within 15 at the leg's start")
first_hit = {}
for h in sorted(w.hits, key=lambda h: h["t"]):
    first_hit.setdefault(h["uid"], h["t"])
idle_len, idle_pause, idle_radius = [], [], []
for e in monsters:
    segs = [s for s in e.segments if s[1] > s[0]]
    home = (segs[0][2], segs[0][3]) if segs else (e.x, e.y)
    prev_end = None
    for s in segs:
        t0 = s[0]
        if t0 >= first_hit.get(e.uid, math.inf):
            break
        near = any(seen(p, t0) and dist(pos(p.uid, t0), (s[2], s[3])) < 15 for p in players)
        if near:
            prev_end = None
            continue
        idle_len.append(dist((s[2], s[3]), (s[4], s[5])))
        idle_radius.append(dist((s[4], s[5]), home))
        if prev_end is not None:
            idle_pause.append(t0 - prev_end)
        prev_end = s[1]
print(f"  leg length {q(idle_len)}")
print(f"  pause between legs {q(idle_pause)}")
print(f"  leg end from the first position {q(idle_radius)}")

print("\n== E. Alarm: a monster's first leg aimed at a player (destination within 4 of the player), no hit on it before")
AIM = 4.0


def aimed_at(e, s):
    """The player a leg aims at: destination within AIM of the player's position at the start."""
    best, bd = None, AIM
    for p in players:
        if seen(p, s[0]):
            d = dist(pos(p.uid, s[0]), (s[4], s[5]))
            if d < bd:
                best, bd = p, d
    return best


alarm_d, alarm_hit_first = [], 0
chases = []  # (monster, player, [legs])
for e in monsters:
    segs = [s for s in e.segments if s[1] > s[0]]
    cur_p, cur_legs = None, []
    for s in segs:
        p = aimed_at(e, s)
        if p is not None and (cur_p is None or p.uid == cur_p.uid):
            cur_legs.append(s)
            cur_p = p
        else:
            if cur_p is not None and cur_legs:
                chases.append((e, cur_p, cur_legs, s))
            cur_p, cur_legs = (p, [s]) if p is not None else (None, [])
    if cur_p is not None and cur_legs:
        chases.append((e, cur_p, cur_legs, None))
seen_first = set()
for e, p, legs, _ in chases:
    if e.uid in seen_first:
        continue
    seen_first.add(e.uid)
    t0 = legs[0][0]
    if t0 < first_hit.get(e.uid, math.inf):
        alarm_d.append(dist((legs[0][2], legs[0][3]), pos(p.uid, t0)))
    else:
        alarm_hit_first += 1
print(f"  distance monster to player at the first aimed leg, unprovoked: {q(alarm_d)}; monsters whose first aimed leg came after a hit: {alarm_hit_first}")

print("\n== F. Chases: monster-to-target distance at each chase leg's start, and at the leg a chase ended on")
leg_d, giveup_d, chase_len = [], [], []
for e, p, legs, after in chases:
    ds = [dist((s[2], s[3]), pos(p.uid, s[0])) for s in legs]
    leg_d.extend(ds)
    chase_len.append(legs[-1][1] - legs[0][0])
    if after is not None and e.dead_at is None or (after is not None and e.dead_at and after[0] < e.dead_at):
        giveup_d.append(ds[-1])
print(f"  chases {len(chases)}; distance at a chase leg's start {q(leg_d)}")
print(f"  distance at the last leg of a chase that ended with the monster alive and walking elsewhere {q(giveup_d)}")
print(f"  chase duration {q(chase_len)}")

print("\n== G. Monsters' attacks on players (bites)")
mcasts = [c for c in w.casts if c["caster_kind"] == "Monster" and c["target_kind"] == "Player"]
by_pair = defaultdict(list)
for c in mcasts:
    by_pair[(c["caster"], c["target"])].append(c["t"])
intervals = [b - a for ts in by_pair.values() for a, b in zip(ts, ts[1:]) if b - a < 10]
print(f"  casts {len(mcasts)}; interval between a monster's casts at the same player (under 10 s) {q(intervals, 2)}")
print(f"  distance at the cast {q([c['distance'] for c in mcasts if c.get('distance') is not None])}")
dmg_by_ref = defaultdict(list)
for h in w.hits:
    if h["victim_kind"] == "Player" and h["caster"] is not None and h["caster"] in ents and ents[h["caster"]].kind == "Monster":
        e = ents[h["caster"]]
        dmg_by_ref[(e.ref_id, e.rarity)].append(h["damage"])
for (ref, rar), v in sorted(dmg_by_ref.items(), key=lambda kv: -len(kv[1])):
    print(f"  model {ref} rarity 0x{rar:02x}: damage per hit on a player {q(v, 0)}")
mskills = Counter((c["caster_ref"], c["skill"]) for c in mcasts)
print(f"  monster skills: {mskills.most_common(8)}")
print(f"  the observer's own HP: max {max(m['hp'] for m in w.me if m['hp'])}, min {min(m['hp'] for m in w.me if m['hp'])}")

print("\n== H. Threat: lured monsters' first attack target, and the switch after the first dps hit")
hits_by_uid = defaultdict(list)
for h in sorted(w.hits, key=lambda h: h["t"]):
    if h["caster"] is not None:
        hits_by_uid[h["uid"]].append(h)
casts_by_monster = defaultdict(list)
for c in mcasts:
    casts_by_monster[c["caster"]].append(c)
lured = 0
attacked_lurer_first = 0
switch_delay = []
no_switch = 0
first_cast_delay = []
for uid, hs in hits_by_uid.items():
    if uid not in ents or ents[uid].kind != "Monster":
        continue
    if hs[0]["caster"] not in LURERS:
        continue
    lured += 1
    cs = casts_by_monster.get(uid, [])
    after = [c for c in cs if c["t"] >= hs[0]["t"]]
    if after:
        first_cast_delay.append(after[0]["t"] - hs[0]["t"])
        if after[0]["target"] in LURERS:
            attacked_lurer_first += 1
    dps_hit = next((h for h in hs if h["caster"] in DPS), None)
    if dps_hit:
        sw = next((c for c in cs if c["t"] > dps_hit["t"] and c["target"] not in LURERS), None)
        if sw:
            switch_delay.append(sw["t"] - dps_hit["t"])
        else:
            no_switch += 1
print(f"  monsters whose first hit seen was a lurer's: {lured}; first attack after the hit aimed at a lurer: {attacked_lurer_first}; delay hit to first attack {q(first_cast_delay)}")
print(f"  after the first dps hit: switched to a non-lurer target {len(switch_delay)} (delay {q(switch_delay)}), never seen switching {no_switch}")

print("\n== I. The lurers' kit")


def skill_row(sk):
    r = w.skills.get(sk, {})
    return f"{r.get('name', '?')} range {r.get('range')} cooldown {r.get('cooldown')} cast {r.get('cast_time')} mp {r.get('mp')}"


for u in LURERS:
    cs = [c for c in w.casts if c["caster"] == u]
    print(f"  {party[u][0]}: {len(cs)} casts")
    for sk, n in Counter(c["skill"] for c in cs).most_common():
        sc = [c for c in cs if c["skill"] == sk]
        d = [c["distance"] for c in sc if c.get("distance") is not None and c["target_kind"] == "Monster"]
        nh = [c["nhits"] for c in sc]
        ts = sorted(c["t"] for c in sc)
        iv = [b - a for a, b in zip(ts, ts[1:])]
        print(f"    skill {sk} x{n}: {skill_row(sk)}; hits per cast mean {st.mean(nh):.2f} max {max(nh)}; distance to target {q(d)}; interval {q(iv)}")

print("\n== J. The dps")
for u in DPS:
    cs = [c for c in w.casts if c["caster"] == u]
    ts = sorted(c["t"] for c in cs)
    iv = [b - a for a, b in zip(ts, ts[1:]) if b - a < 10]
    print(f"  {party[u][0]}: {len(cs)} casts; interval between casts (under 10 s) {q(iv, 2)}")
    for sk, n in Counter(c["skill"] for c in cs).most_common(4):
        sc = [c for c in cs if c["skill"] == sk]
        nh = [len(cast_hits(c)) for c in sc]
        frac = []
        spread = []
        absolute = []
        for c in sc:
            tpos = c.get("my_target_xy")
            for h in cast_hits(c):
                v = ents.get(h[0])
                if v and v.kind == "Monster" and v.max_hp and v.ref_id in TOP_MODELS and v.rarity == 0:
                    frac.append(100 * h[1] / v.max_hp)
                    absolute.append(h[1])
                if v and v.kind == "Monster" and v.max_hp:
                    if tpos and h[0] != c["target"]:
                        spread.append(dist(pos(h[0], c["t"]), tpos))
        d = [c["distance"] for c in sc if c.get("distance") is not None]
        print(f"    skill {sk} x{n}: {skill_row(sk)}; hits per cast mean {st.mean(nh):.2f} max {max(nh)}; damage on a normal monster of the commonest models ({TOP_HP} HP) {q(absolute, 0)} = % of max HP {q(frac)}; other hit targets' distance from the main one {q(spread)}; cast distance {q(d)}")

print("\n== K. The buffers")
role_of = lambda uid: "none (0)" if uid == 0 else party.get(uid, ("", "other"))[1]
for u in BUFFERS:
    cs = [c for c in w.casts if c["caster"] == u]
    ts = sorted(c["t"] for c in cs)
    iv = [b - a for a, b in zip(ts, ts[1:]) if b - a < 30]
    print(f"  {party[u][0]} ({party[u][1]}): {len(cs)} casts; interval {q(iv, 2)}")
    for sk, n in Counter(c["skill"] for c in cs).most_common(8):
        sc = [c for c in cs if c["skill"] == sk]
        tr = Counter(role_of(c["target"]) if c["target"] != u else "self" for c in sc)
        durs = [b["duration"] for b in w.buffs if b["skill"] == sk and b["duration"] is not None and b["target"] in party]
        print(f"    skill {sk} x{n}: {skill_row(sk)}; targets {dict(tr)}; duration {q(durs)}")

print("\n== L. Monsters seen, by rarity, and respawns")
rar = Counter(e.rarity for e in monsters)
print(f"  distinct monsters {len(monsters)}: by rarity byte {dict(sorted(rar.items()))}")
births = [e for e in monsters if not e.first_group]
print(f"  single spawns (group=false) {len(births)}, by rarity {dict(sorted(Counter(e.rarity for e in births).items()))}")
resp = []
for d in w.deaths:
    if d["kind"] != "Monster":
        continue
    p = pos(d["uid"], d["t"])
    ref = d["ref_id"]
    nxt = min((e.first_seen - d["t"] for e in births if e.ref_id == ref and e.first_seen > d["t"] and dist((e.segments[0][2], e.segments[0][3]) if e.segments else (e.x, e.y), p) < 20), default=None)
    if nxt is not None:
        resp.append(nxt)
print(f"  death to the next single spawn of the same model within 20: {q(resp)}")
gd = [dist((e.segments[0][2], e.segments[0][3]) if e.segments else (e.x, e.y), OBS) for e in monsters if e.first_group]
sd = [dist((e.segments[0][2], e.segments[0][3]) if e.segments else (e.x, e.y), OBS) for e in births]
print(f"  first position from the observer: group spawns {q(gd)}; single spawns {q(sd)}")

print("\n== M. The lurers' trips (leave: beyond OUT of the centre; back: within IN)")
OUT, IN = 25.0, 15.0
for u in LURERS:
    e = ents[u]
    track = [(t, pos(u, t)) for t in [x / 2 for x in range(0, int(END * 2))] if seen(e, t)]
    trips = []
    out_t = None
    far = 0.0
    farp = None
    for t, p in track:
        d = dist(p, centre)
        if out_t is None and d > OUT:
            out_t, far, farp = t, d, p
        elif out_t is not None:
            if d > far:
                far, farp = d, p
            if d < IN:
                trips.append((out_t, t, far, farp))
                out_t = None
    rows = []
    rows_delivered = []
    lurer_hits = [h for h in w.hits if h["caster"] == u and h["victim_kind"] == "Monster"]
    for k, (a, b, far, farp) in enumerate(trips):
        tagged = {h["uid"] for h in lurer_hits if a <= h["t"] <= b}
        casts = sum(1 for c in w.casts if c["caster"] == u and a <= c["t"] <= b)
        # Following at the return: tagged monsters alive within 12 of the lurer at the return.
        following = sum(1 for m in tagged if m in ents and seen(ents[m], b) and dist(pos(m, b), pos(u, b)) < 12)
        delivered = 0
        tag_to_death = []
        for m in tagged:
            d = next((d for d in w.deaths if d["uid"] == m and d["t"] >= a), None)
            if d and d.get("last_caster") in party and d["t"] - a < 120:
                delivered += 1
                first_tag = min(h["t"] for h in lurer_hits if h["uid"] == m and h["t"] >= a)
                tag_to_death.append(d["t"] - first_tag)
            elif m in ents and any(seen(ents[m], tt) and dist(pos(m, tt), centre) < 20 for tt in range(int(a), int(min(END, a + 120)))):
                delivered += 1
        rows_delivered.append((delivered, tag_to_death))
        dwell = (trips[k + 1][0] - b) if k + 1 < len(trips) else None
        # Still chased at the next leave: a tagged monster of this trip casting at the lurer within 3 s before it.
        chased = None
        if k + 1 < len(trips):
            nl = trips[k + 1][0]
            chased = any(c["target"] == u and nl - 3 <= c["t"] <= nl for c in mcasts)
        rows.append((a, b, far, len(tagged), casts, following, dwell, chased))
    print(f"  {party[u][0]}: {len(trips)} trips")
    print(f"    trip duration {q([r[1]-r[0] for r in rows])}")
    print(f"    farthest point from the centre {q([r[2] for r in rows])}")
    print(f"    monsters tagged per trip {q([r[3] for r in rows], 0)}; casts per trip {q([r[4] for r in rows], 0)}")
    print(f"    tagged monsters within 12 at the return {q([r[5] for r in rows], 0)}")
    print(f"    delivered per trip (tagged, then killed by the party or inside 20 of the centre within 120 s) {q([r[0] for r in rows_delivered], 0)}; total {sum(r[0] for r in rows_delivered)}; tag to death {q([x for r in rows_delivered for x in r[1]])}")
    print(f"    trips with no tag at all: {sum(1 for r in rows if r[3] == 0)} of {len(rows)}")
    print(f"    dwell at the centre before the next trip {q([r[6] for r in rows if r[6] is not None])}")
    print(f"    next leave while a tagged monster still attacked it (3 s): {sum(1 for r in rows if r[7])} of {sum(1 for r in rows if r[7] is not None)}")
    inside = sum(1 for t, p in track if dist(p, centre) < IN) / max(1, len(track))
    print(f"    time within {IN} of the centre: {100*inside:.0f}%")
    for r in rows[:6]:
        print(f"      trip {r[0]:.0f}-{r[1]:.0f} s far {r[2]:.0f} tagged {r[3]} casts {r[4]} following {r[5]} dwell {r[6]} chased-at-leave {r[7]}")

print("\n== Q. Monsters alive within 20 of the centre, sampled each second (the dps disk's load), and party monsters among them")
load, strong = [], []
for t in range(0, int(END)):
    if not clean(t):
        continue
    inside = [m for m in monsters if seen(m, t) and dist(pos(m.uid, t), centre) < 20]
    load.append(len(inside))
    strong.append(sum(1 for m in inside if m.rarity >> 4 == 1 or m.rarity & 0x0f in (4, 5)))
print(f"  monsters inside {q(load, 0)}; party monsters or giants inside {q(strong, 0)}; seconds with none inside {sum(1 for x in load if x == 0)} of {len(load)}")
hp_inside = []
dmg_so_far = defaultdict(int)
hit_times = sorted(w.hits, key=lambda h: h["t"])
k = 0
for t in range(0, int(END)):
    while k < len(hit_times) and hit_times[k]["t"] <= t:
        dmg_so_far[hit_times[k]["uid"]] += hit_times[k]["damage"]
        k += 1
    if not clean(t):
        continue
    hp_inside.append(sum(max(0, m.max_hp - dmg_so_far[m.uid]) for m in monsters if seen(m, t) and dist(pos(m.uid, t), centre) < 20))
print(f"  HP left inside (max HP minus damage seen), in normal monsters of {TOP_HP}: {q([x / max(TOP_HP, 1) for x in hp_inside])}")

print("\n== N. Sight: lurer walks ending at a monster, and the monster's distance from where the walk began")
sight = []
for u in LURERS:
    for s in ents[u].segments:
        if s[1] <= s[0]:
            continue
        for m in monsters:
            if seen(m, s[0]) and dist(pos(m.uid, s[0]), (s[4], s[5])) < 4:
                sight.append(dist(pos(m.uid, s[0]), (s[2], s[3])))
                break
print(f"  {q(sight)}")

print("\n== O. Player deaths and resurrections")
pdeaths = [d for d in w.deaths if d["kind"] == "Player"]
print(f"  player deaths {len(pdeaths)}: {[(round(d['t']), ents[d['uid']].name if d['uid'] in ents else d['uid'], role_of(d['uid'])) for d in pdeaths]}")
for d in pdeaths:
    res = [c for c in w.casts if c["target"] == d["uid"] and c["t"] > d["t"] and c["caster_kind"] == "Player" and c["caster"] != d["uid"]]
    if res:
        c = res[0]
        print(f"    first cast on the dead {c['caster_name']} skill {c['skill']} {skill_row(c['skill'])} after {c['t']-d['t']:.1f} s")

print("\n== P. Hits per cast over all players, by skill (the area cap)")
top = Counter()
for c in w.casts:
    if c["caster_kind"] == "Player":
        top[c["skill"]] = max(top[c["skill"]], len(cast_hits(c)))
print("  max hits per cast:", sorted(top.items(), key=lambda kv: -kv[1])[:8])
