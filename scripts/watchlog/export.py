"""Export a watch log window into CSV tables (section 4 of CARAVAN_LOGGER_REQUIREMENTS.md)."""
import csv
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import watchlog

parser = argparse.ArgumentParser(description='Export a watch log window into CSV tables (entities, tracks, casts, hits, buffs, deaths, me, npcs, skills).')
watchlog.common_args(parser)
parser.add_argument("--out", required=True, help="a folder outside git for the tables (they hold other players' names)")
args = parser.parse_args()
w = watchlog.load_args(args)
out = Path(args.out)
out.mkdir(parents=True, exist_ok=True)


def write(name, rows, fields):
    with open(out / name, "w", newline="") as f:
        wr = csv.DictWriter(f, fieldnames=fields, extrasaction="ignore")
        wr.writeheader()
        wr.writerows(rows)


ents = w.all_entities()
write("entities.csv", [dict(uid=e.uid, kind=e.kind, ref_id=e.ref_id, name=e.name, rarity=e.rarity, max_hp=e.max_hp,
                            walk=e.walk, run=e.run, first_seen=e.first_seen, last_seen=e.last_seen, dead_at=e.dead_at,
                            group=getattr(e, "spawn_group", None), reason=getattr(e, "despawn_reason", None))
                       for e in ents],
      ["uid", "kind", "ref_id", "name", "rarity", "max_hp", "walk", "run", "first_seen", "last_seen", "dead_at", "group", "reason"])
segs = []
for e in ents:
    for s in e.segments:
        segs.append(dict(uid=e.uid, t0=s[0], t1=s[1], x0=s[2], y0=s[3], x1=s[4], y1=s[5], speed=s[6], motion=s[7]))
write("tracks.csv", segs, ["uid", "t0", "t1", "x0", "y0", "x1", "y1", "speed", "motion"])
write("casts.csv", [dict(c, caster_my_x=(c["my_caster_xy"] or (None, None))[0], caster_my_y=(c["my_caster_xy"] or (None, None))[1],
                         target_my_x=(c["my_target_xy"] or (None, None))[0], target_my_y=(c["my_target_xy"] or (None, None))[1],
                         nhits=c.get("nhits", 0), teleport=c.get("teleport", ""), flags=c.get("flags"))
                    for c in w.casts],
      ["t", "i", "caster", "caster_name", "caster_kind", "caster_ref", "skill", "target", "target_name", "target_kind", "target_ref",
       "action", "flags", "nhits", "distance", "caster_wx", "caster_wy", "target_wx", "target_wy",
       "caster_my_x", "caster_my_y", "target_my_x", "target_my_y", "teleport"])
write("hits.csv", w.hits, ["t", "cast_index", "action", "caster", "skill", "uid", "damage", "crit", "blocked", "dead", "victim_kind", "victim_ref", "victim_name"])
write("buffs.csv", w.buffs, ["target", "skill", "token", "t_on", "t_off", "duration"])
write("deaths.csv", w.deaths, ["t", "uid", "kind", "ref_id", "name", "last_caster", "last_skill"])
write("me.csv", w.me, ["t", "hp", "mp", "attacked_by", "untouchable", "dead"])
write("npcs.csv", w.npcs.values(), ["ref_id", "code", "name", "level", "max_hp", "walk", "run", "rarity_col"])
write("skills.csv", w.skills.values(), ["skill", "code", "name", "range", "cooldown", "cast_time", "mp", "target_self", "target_ally", "target_party", "target_enemy"])
print(f"end {w.end:.1f} s; entities {len(ents)}; segments {len(segs)}; casts {len(w.casts)}; hits {len(w.hits)}; buffs {len(w.buffs)}; deaths {len(w.deaths)}; me {len(w.me)}; index errors {w.index_errors}")
print("roles:", w.session.get("roles"))
