"""Section 7 of CARAVAN_LOGGER_REQUIREMENTS.md: the acceptance checks over a watch log window."""
import math
import argparse
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import watchlog

parser = argparse.ArgumentParser(description='Section 7 of CARAVAN_LOGGER_REQUIREMENTS.md over a watch log window.')
watchlog.common_args(parser)
args = parser.parse_args()
w = watchlog.load_args(args)
OBS = (w.session["wx"], w.session["wy"])

# Attacked stretches (R10): from a me line with attacked_by set to the next without.
attacked = []
start = None
for m in w.me:
    if m["attacked_by"] and start is None:
        start = m["t"]
    elif not m["attacked_by"] and start is not None:
        attacked.append((start, m["t"]))
        start = None
if start is not None:
    attacked.append((start, w.end))
cut = sum(b - a for a, b in attacked)
print(f"me lines {len(w.me)}, attacked {sum(1 for m in w.me if m['attacked_by'])}, stretches {len(attacked)}, cut {cut:.0f} s of {w.end:.0f}")


def in_attacked(t):
    return any(a <= t < b for a, b in attacked)


# 1. Tracks against the derived positions, within 2 units.
def check1(exclude_attacked):
    n = ok = tele_miss = 0
    worst = []
    for c in w.casts:
        if exclude_attacked and in_attacked(c["t"]):
            continue
        for who in ("caster", "target"):
            dx, dy = c.get(f"{who}_wx"), c.get(f"{who}_wy")
            mine = c[f"my_{who}_xy"]
            if dx is None or mine is None:
                continue
            n += 1
            d = math.hypot(mine[0] - dx, mine[1] - dy)
            if d <= 2:
                ok += 1
            else:
                worst.append((d, c["t"], who, c["caster_name"], c["skill"], c.get("flags")))
                if who == "caster" and c.get("teleport"):
                    tele_miss += 1
    worst.sort(reverse=True)
    return n, ok, tele_miss, worst[:5]


n, ok, tele_miss, worst = check1(False)
print(f"check 1: {ok} of {n} positions within 2 ({100*ok/n:.1f}%); misses {n-ok}, of them teleporting casters {tele_miss}")
print("   worst:", [(round(d, 1), round(t, 1), who, name, sk, fl) for d, t, who, name, sk, fl in worst])
casters_only = sum(1 for c in w.casts if c["my_caster_xy"] and c.get("caster_wx") is not None)
casters_ok = sum(1 for c in w.casts if c["my_caster_xy"] and c.get("caster_wx") is not None
                 and math.hypot(c["my_caster_xy"][0] - c["caster_wx"], c["my_caster_xy"][1] - c["caster_wy"]) <= 2)
print(f"   casters only: {casters_ok} of {casters_only} ({100*casters_ok/casters_only:.1f}%)")

# 2. Damage reaches max HP within one hit, monsters with a life 2.
dmg = defaultdict(int)
biggest = defaultdict(int)
for h in w.hits:
    dmg[h["uid"]] += h["damage"]
    biggest[h["uid"]] = max(biggest[h["uid"]], h["damage"])
ents = {e.uid: e for e in w.all_entities()}
near = far = near_ok = far_ok = 0
short_near = []
for d in w.deaths:
    e = ents.get(d["uid"])
    if not e or e.kind != "Monster":
        continue
    first = watchlog.pos_from_segments(e.segments, e.first_seen, (e.x, e.y))
    dist = math.hypot(first[0] - OBS[0], first[1] - OBS[1])
    reached = e.max_hp - dmg[d["uid"]] <= biggest[d["uid"]]
    if dist <= 40:
        near += 1
        near_ok += reached
        if not reached:
            short_near.append((e.uid, e.ref_id, e.rarity, e.max_hp, dmg[d["uid"]], round(dist), getattr(e, "spawn_group", None), round(e.first_seen)))
    else:
        far += 1
        far_ok += reached
print(f"check 2: within 40 of the observer {near_ok} of {near}; beyond {far_ok} of {far}")
print("   short near (uid, ref, rarity, max_hp, damage, dist, group, first_seen):", short_near[:8])

# 3. Buff durations.
timed = [b for b in w.buffs if b["duration"] is not None]
print(f"check 3: {len(timed)} timed of {len(w.buffs)} buff_off; unmatched tokens {len(w.buffs) - len(timed)}")

# 5. Index order and gaps.
print(f"check 5: index errors {w.index_errors}")
