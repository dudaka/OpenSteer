# Requirements: a Caravan logger for the Training Field

Written 2026-11-04 for the Caravan project (`~/Workspace/silkroad/caravan`, read here read-only).
Purpose: replace the Training Field's assumed numbers (`crates/steer-demos/src/training_field.rs`,
the constants at the top of the module and `PLAN.md` Phase 8 "As built") with numbers measured
from a real training party. The setup is the user's: one Caravan character is logged in and
stands at the training disk of an 8-player party (2 lurers, 4 dps, 2 buffers) that is training,
and records what the server shows it. The analysis is done afterwards in this repo (a scratch
`uv run` script over the log), never in Caravan.

Everything below names Caravan's existing types and packets so the implementer knows what is
already parsed; where the logger lives in Caravan's layers is Caravan's decision (the natural
hook is the bot's read loop, beside `watch_step` in `caravan-bot/src/lib.rs`, which already logs
walks, casts and deaths as `bot.watch` lines to `caravand.log`). This document asks for a
separate machine-readable session file, not more text lines.

## 1. Three facts that already correct the demo

Found while reading Caravan; each is a measurement the logger must make precise.

1. **The server sends no HP for a monster the character has not selected** (Caravan D103;
   `WorldState` keeps `hp` only for `target`). A player sees one monster's HP bar at a time.
   The demo's lurer reads every monster bar in the dps disk (`Lure::backlog`); the real one
   cannot. A monster's HP left is only known as its max HP (gamedata x the rarity multiplier)
   minus the damage of every hit seen on it (0xB070 / 0xB071 hits), which is what the logger
   must make reconstructible.
2. **The party window gives HP and MP in tenths** (`party::Member::hp_mp`: HP tenths in the
   low nibble, MP in the high one; 0x3864 type 6 deltas). The demo's risk check reads 70%, 50%,
   a fall of 15 or 30 points and the healer's 50 MP as exact bars; the real resolution is 10%.
   The update cadence is not documented anywhere in Caravan: the logger records every delta
   with its time so it can be measured.
3. **The party window carries every member's position** (`Member::region, x, y, z`,
   `MemberDelta::position`; that they keep coming beyond sight is inferred from the struct and
   the game's minimap, not yet seen in a Caravan log). The demo forbids a lurer any position beyond 30;
   a real lurer knows where the other lurer is from the party window (at whatever cadence the
   deltas come). This is only available if the observer is a party member (section 3).

Also already known, so the logger need not establish them: the server shows the 3x3 cells of
320 wire round the character (everything within 32 world units for sure, nothing beyond about
90; `docs/conventions.md` § Movement gotchas, measured on players, "monsters not measured"),
wire positions are tenths of a world unit, wire speeds x 0.1 are world units a second, the
game's angle is 65536 a full turn with 0 east counter-clockwise, and world coordinates are
`((region & 0xFF) - 135) * 192 + x / 10`, `((region >> 8) - 92) * 192 + y / 10`.

## 2. What the server shows, and what Caravan already parses

| Packet | Caravan type | What the logger takes from it |
|---|---|---|
| 0x3015 / 0x3016 single spawn / despawn; 0x3017-0x3019 group spawns (cell changes) | `Entity::{Player, Monster, Cos}`, `BionicPose` | uid, kind, `ref_id`, rarity byte, position, angle, walk / run / berserk speed, destination if walking, buffs, life state; players: name, guild, equipment, `in_combat` |
| 0xB021 walk | `EntityMovement` | uid, source, destination or angle-only (endless), movement type |
| 0xB023 position, 0xB024 angle | `EntityPosition`, `EntityAngle` | uid, position / angle (the server's corrections and stops) |
| 0xB0D1 speed | `SpeedUpdate` | uid, walk and run speed |
| 0x30BF state | `StateChange` | uid, axis (0 life: 1 alive, 2 dead; 1 motion: 2 walk, 3 run, 4 sit; 4 body: 2 untouchable), value |
| 0xB070 cast (flag 1) | `SkillCast`, `SkillHit` | skill id (the executed MSKILL), caster, target, action id, one hit per affected uid: damage, critical, blocked; knockback positions; teleport landing. **The dead flag (0x80 of the hit state) is dropped by `SkillHit` today: the logger needs it** |
| 0xB071 cast end | `SkillEnd` | action id, target, result, hits (a cast-time skill's damage arrives here) |
| 0xB074 action state | `ActionState` | ours only (begun / ended / cancelled) |
| 0xB0BD buff on, 0xB072 buffs off | `BuffAdd`, `BuffRemove` | target, skill id, token; the tokens ended (no uid: the logger resolves token to target) |
| 0x3057 status | `EntityStatus` | uid, HP, MP, bad status; arrives for us and for our selected target only |
| 0x3065 party, 0x3864 party updates | `PartyData`, `Member`, `MemberDelta` | members: uid, name, level, HP / MP tenths, position, masteries; joins, leaves, deltas |
| 0x3026 chat | `ChatLine` | channel, sender, text (optional, section 4 R9) |
| gamedata.db `npcs` | `GameData` | per `ref_id`: code name, name, level, `max_hp`, `walk_speed`, `run_speed`, rarity column |
| gamedata.db `skills` | `SkillRow` | per skill id: code name, name, `range`, `cooldown`, `cast_time`, `mp_cost`, targets, raw `params` |

Rarity byte (`views::hp_multiplier`, the client's rule): low nibble 1 champion x2, 4 giant
x20, 5 titan x100, 6 elite x4, 7 strong elite x30, 3 and 8 unique, else x1; high nibble 1 is
a party monster, x10 more. The demo's HP multipliers (x1 / x2 / x20 / x10 / x20 / x200) came
from this table; its damage multipliers (x1 / x1.5 / x3 / x2 / x3 / x6) are ours and must be
measured.

## 3. The observer

- **One Caravan character, clientless** (a VIP account: the non-VIP probe disconnects at about
  212 s, Caravan D100), standing still at a fixed point inside or at the edge of the party's
  training disk. It sends nothing but what keeps it logged in; it attacks nobody, uses no
  skill and takes no item. If a monster attacks it, that is recorded (R10) and the stretch is
  excluded from the analysis.
- **Setup A, bystander** (decided 2026-11-04: the observer is not in the party). It sees
  every entity in the 3x3 cells, every walk, cast, hit, buff and death, but no player's HP or
  MP (the server sends none for others) and no position beyond the cells.
- **Setup B, member**, not used: the observer would have to be one of the eight (a party is
  full at eight with a sharing flag, `party::capacity`). Only that setup gives the party window
  (HP / MP tenths and positions of the others), so the rows marked "setup B" in section 5 are
  not measurable in these sessions: buff and bite effects are measured in casts and gamedata
  only. The party records of R9 stay specified in case a later session is a member's.
- The session header (R3) records the setup, so the analysis knows what is absent.

## 4. Logger requirements

R1. **Where the records go (recommended: the existing log).** `caravand.log` already holds
    `bot.watch` lines (walks, players' casts with the uids hit, monsters' casts at players,
    monsters' deaths), the D105 analysis ran over them, and this repo can read the file
    directly (`~/.caravan/caravand.log`). The smallest logger is therefore more `bot.watch`
    lines of the same form (`key=value` fields, one event a line, the `tracing` timestamp),
    with the fields and events below added (decided 2026-11-04). The record shapes in
    section 4 are the field lists; a separate JSON Lines file is only a fallback if the text
    lines get unwieldy.

R2. **Every record carries a time and an order.** In `caravand.log` the `tracing` timestamp
    (microseconds) is the time; add `i`, the packet's index since the login, so order within a
    timestamp is kept. Times are the receive time of the packet; nothing is back-dated.

R3. **A header record first** (`ev: "session"`): account, the observer's uid and name, the
    setup (A or B), the standing point (region, wire x / y / z and world x / y), the party id
    if known, the party's members as the user names them (name to role: lurer / dps / hp
    buffer / mp buffer; typed in by the user when the session starts, since no packet says a
    role), caravand's version, gamedata's version or file hash, the server (gateway host).

R4. **Definition records at first sight** (`ev: "def_npc"`, `ev: "def_skill"`): the gamedata
    row of every `ref_id` and every skill id the session meets, written once each, so the file
    is self-contained (name, level, max HP, walk / run speed, rarity column; skill code name,
    name, range, cooldown, cast time, MP cost, target flags, raw `params`).

R5. **Entities.** A record per spawn and despawn (single and group alike, with a flag saying
    which: a group burst is a cell change, a single one a birth, a death's body or a logout):
    uid, kind, `ref_id`, the raw rarity byte and the decoded max HP, position (region and wire
    x / y / z, plus world x / y), angle, walk / run / berserk speed (wire and world units a
    second), life state, the buffs carried, the destination if already walking; players: name,
    guild, job, `in_combat`, equipment item ids (the weapon tells the role when the user's list
    is missing). Despawns: uid and the reason Caravan can tell (a death seen, else unknown).

R6. **Movement, raw.** A record per 0xB021 (uid, source if given, destination or the angle of
    an endless walk, movement type), per 0xB023 and 0xB024, per 0xB0D1, per motion change of
    0x30BF. No interpolation in the log. Optionally (a flag, decided off) a `snap` record once a
    second per living entity in sight with Caravan's estimated position (`pos_at`), marked
    `derived: true`, so a plot needs no replay. Decided off (2026-11-04): the analysis
    rebuilds positions from the walks, as the D105 analysis did.

R7. **Combat.** A record per 0xB070 flag 1 and per 0xB071: skill id, caster, target, action
    id, result, the hits (uid, damage, critical, blocked, **dead**), knockback positions, the
    teleport landing; plus, marked `derived: true`, the caster's and the target's estimated
    positions at receipt and their distance, so a skill's reach and an area's spread are
    measurable without replaying the walks. Our own 0xB074 and 0xB070 flag 2 refusals too
    (they should not occur: the observer acts on nothing).

R8. **Buffs.** A record per 0xB0BD (target, skill id, token) and per 0xB072 (each token, with
    the target the logger resolved from its 0xB0BD), so durations and who buffs whom are
    measurable.

R9. **Life and bars.** A record per 0x30BF life change (deaths and resurrections of players
    and monsters; for a death, the uid and skill of the last hit seen on it, `derived: true`),
    per 0x3057 (ours only: the observer selects nothing, decided 2026-11-04), and in setup B
    per 0x3065 and per 0x3864 (type, member uid, and for type 6 every field present: HP / MP
    tenths, position, level). No chat lines (decided 2026-11-04).

R10. **The observer's own state** once a second: position, HP, MP, whether anything attacks
    it (a 0xB070 with our uid as the target), whether it is untouchable or dead. A stretch
    where it is attacked, moved or dead is cut from the analysis.

R11. **Durability.** Append-only, flushed at least every 100 ms, a `gap` record on a
    disconnect or a silence over 5 s and a new header after a re-login (or a new file), an
    `end` record on stop. A crash loses at most the last 100 ms.

R12. **Raw payloads, optional.** For 0xB070 and 0xB071 (the packets with unparsed tails) the
    hex payload beside the parsed fields, so a mis-parse found later can be re-read and pinned
    as a fixture. Decided off (2026-11-04); worth a flag only if the hit numbers look wrong.

R13. **Size.** Eight players and 15-40 monsters give roughly 50-200 records a second at
    100-300 bytes each: under 60 KB/s, under 220 MB for an hour. Acceptable; no compression
    needed in the first version.

R14. **Nothing else in Caravan.** No analysis, no rendering, no view model, no change to the
    bot's behaviour while logging. The numbers are computed here.

### Record shapes

The field list per event, shown as JSON for compactness; as `bot.watch` lines each field is a
`key=value`, `ev` is the line's message and the `tracing` timestamp replaces `t` and `wall`. Short, stable names; positions as raw wire
(`region, x, y, z`) and world (`wx, wy`); every value derived by Caravan rather than read from
the wire carries `derived: true`.

```
{"ev":"session","t":0,"wall":"2026-11-05T01:17:03.120Z","i":0,"account":"...","me":{"uid":93018,"name":"..."},"setup":"A","stand":{"region":21024,"x":12800,"y":16300,"z":0,"wx":-640.0,"wy":-97.0},"party_id":null,"roles":{"MacCongTu":"lurer","magics":"lurer","...":"dps"},"caravand":"0.1.10","gamedata":"sha256:...","server":"gw2sro.vtcgame.vn"}
{"ev":"def_npc","t":12,"i":3,"ref_id":1933,"code":"MOB_CH_...","name":"Sâu cát","level":101,"max_hp":37844,"walk":16,"run":60,"rarity_col":0}
{"ev":"def_skill","t":12,"i":3,"skill":7146,"code":"SKILL_EU_WARRIOR_FRENZYA_TOUNT_A_17","name":"Taunting Target","range":150,"cooldown":5000,"cast_time":0,"mp":303,"params":"78|0|6645362|..."}
{"ev":"spawn","t":12,"i":3,"uid":464001,"kind":"Monster","ref_id":1933,"rarity":16,"max_hp":378440,"group":true,"region":21024,"x":13010,"y":16122,"z":0,"wx":-619.0,"wy":-114.8,"angle":12000,"walk":1.6,"run":6.0,"berserk":9.0,"life":1,"buffs":[],"dest":null}
{"ev":"despawn","t":90312,"i":5120,"uid":464001,"group":false,"reason":"death"}
{"ev":"walk","t":1210,"i":40,"uid":464001,"from":{"region":21024,"x":13010,"y":16122},"to":{"region":21024,"x":13200,"y":16000},"endless":false,"type":1}
{"ev":"pos","t":1300,"i":41,"uid":464001,"region":21024,"x":13100,"y":16050,"z":0,"angle":12000}
{"ev":"speed","t":1300,"i":42,"uid":464001,"walk":1.6,"run":6.0}
{"ev":"state","t":1300,"i":43,"uid":464001,"axis":"motion","value":3}
{"ev":"cast","t":2200,"i":60,"skill":7146,"caster":70911,"target":464001,"action":9981,"hits":[{"uid":464001,"damage":1,"crit":false,"blocked":false,"dead":false}],"knockback":[],"teleport":null,"derived":{"caster_at":{"wx":-630.0,"wy":-100.0},"target_at":{"wx":-619.0,"wy":-114.8},"distance":18.6}}
{"ev":"cast_end","t":2900,"i":66,"action":9981,"target":464001,"result":1,"hits":[]}
{"ev":"buff_on","t":3000,"i":70,"target":70911,"skill":7216,"token":554433}
{"ev":"buff_off","t":13000,"i":300,"token":554433,"target":70911}
{"ev":"life","t":90300,"i":5119,"uid":464001,"value":2,"last_hit":{"caster":70912,"skill":19419,"t":90250}}
{"ev":"status","t":4000,"i":80,"uid":93018,"hp":12000,"mp":8000}
{"ev":"party","t":5,"i":1,"party_id":4421,"leader":70911,"members":[{"uid":70911,"name":"...","level":110,"hp10":10,"mp10":9,"region":21024,"x":12700,"y":16200,"z":0}]}
{"ev":"party_delta","t":6400,"i":120,"uid":70911,"hp10":8,"mp10":9,"position":null,"level":null}
{"ev":"snap","t":7000,"i":130,"derived":true,"entities":[{"uid":464001,"wx":-612.0,"wy":-110.0,"moving":true}]}
{"ev":"me","t":7000,"i":131,"wx":-640.0,"wy":-97.0,"hp":12000,"mp":8000,"attacked_by":[],"untouchable":false,"dead":false}
{"ev":"gap","t":8000,"i":140,"seconds":6.2,"reason":"silence"}
{"ev":"end","t":1800000,"i":90000,"reason":"stopped"}
```

## 5. What each Training Field assumption maps to

The analysis script computes these from the records; the table is the checklist that the log
must make possible. "Players' HP" rows need setup B.

| Demo constant (assumed) | What to measure | From |
|---|---|---|
| Disks: dps 10, buffers 5, lurers 50; map 60 | The centre (the dps' median position); the dps' and buffers' distance from it over time; the lurers' farthest point per trip; where monsters stand idle | `walk` |
| `RUN_SPEED` 5, `WALK_SPEED` 1.5 (players) | Each player's run / walk speed (spawn and 0xB0D1, wire x 0.1) and the measured speed of its walk legs | `spawn`, `speed`, `walk`, `pos` |
| Monster speed 6 (slider), gait 1.5 walk | Per `ref_id`: walk / run speed; the speed actually used when chasing (motion 2 or 3) | `spawn`, `state`, `walk` |
| `IDLE_RADIUS` 6, gaits 2-6 s | Idle monsters (nobody hit them, nobody near): leg lengths, pauses, the radius round the spawn point | `walk`, `spawn` |
| `ALARM_REACH` 6, attack on sight | The distance between a monster and the nearest player when the monster's first walk toward it starts, with no hit before | `walk`, `cast` |
| `LOS_REACH` 35 (a chaser gives up) | The largest chaser-to-target distance before a chaser turns home; how long after its target left the cells | `walk` |
| `BITE` 3 per 1.5 s, `MELEE_REACH` 2, damage x1 / x1.5 / x3 | Monster casts at players: interval between a monster's casts, damage per hit (as a fraction of the player's HP needs setup B; else absolute per `ref_id` and rarity), distance at the cast | `cast`, `party_delta` |
| `MAX_HP` 100, `REGEN` 1/s | Players' HP / MP tenths over time; the climb while idle | `party`, `party_delta` |
| Threat: the first dps hit takes a monster off the lurer | For each lured monster: the order of hits on it (lurer, dps) against the order of its own cast targets | `cast`, `life` |
| Lurer's kit: tag 8 / 1 s, blast r 3 / 4 s, pierce 12 x 1.5 / 6 s; MP 2 / 6 / 8 | The lurers' skill ids and their gamedata rows; casts per trip; hits per cast; caster-target distance at the cast; interval between casts | `def_skill`, `cast` |
| Dps: 10 damage to 5 targets within 2, every 1.5 s, 6 MP | The dps' skill ids and rows; hits per cast (distribution and maximum); damage per hit over the monster's max HP; the spread of the hit targets' positions; interval between a dps's casts | `def_skill`, `cast` |
| Buffs: +4/s and +3/s for 10 s; heal +40 at 50%; group heal; guard | The buffers' skill ids and rows; who is buffed when (the target's HP tenths at the cast, setup B); durations (`buff_on` to `buff_off`); heal jumps in the tenths | `def_skill`, `cast`, `buff_on`, `buff_off`, `party_delta` |
| `SPAWN_MIX` 80 / 12 / 3 / 4 / 1 / 0, `MONSTER_RESPAWN` 3 s | Spawns by rarity byte per `ref_id`; time from a death to the next spawn of that `ref_id` within 20 world units | `spawn`, `life`, `despawn` |
| `LEAVE_HP` 90%, `BACK_HP` 40%, N 3 chasers, the hand-off | Per lurer trip: HP tenths at departure (setup B); monsters following at the return (walks ending within 10 of the lurer); whether any monster still chased it when it left again; the dwell at the centre | `walk`, `party_delta`, `cast` |
| `LURER_SIGHT` 30, scouting beyond | The farthest monster a lurer walked toward from where it stood when the walk began; monster spawn distances from the observer (the cell rule, unmeasured for monsters) | `walk`, `spawn` |
| `PLAYER_RESPAWN` 5 s, deaths | Player deaths; who resurrected whom (a buffer's cast on a dead uid) and after how long | `life`, `cast` |
| The 1 m scale (Silkroad's 20 / 60 disks drawn as 10 / 50) | A player's run speed in world units a second against the demo's 5; the disks in world units | `speed`, `walk` |

## 6. Sessions to record

- At least 30 minutes of steady training per session, the party's eight roles typed in at
  the start (R3), the spot named (the user's Area 4 at 5248 beside the watched PhBot party
  is a known one), the time of day.
- Three or more sessions, on at least two spots or two parties if possible: the demo's rule
  of never tuning on one seed applies to real parties too.
- One short session (10 minutes) first, to check the file against section 7 before a long one.

## 7. Acceptance

A 10-minute test file passes when a scratch script in this repo can, with no other input:

1. Rebuild every entity's track from `spawn`, `walk`, `pos` and `speed` records and plot it;
   each cast's derived caster and target positions agree with the track within 2 world units.
2. List every cast with its caster's name, skill name, hit count, total damage and the
   caster-target distance, and sum each monster's damage taken up to its `life` 2 record,
   reaching its decoded max HP within one hit's damage for monsters killed in sight.
3. Give each buff's duration from its `buff_on` and `buff_off`.
4. In setup B, give each member's HP and MP tenths as a step function over time.
5. Find no record without `t` and `i`, no `i` out of order, and no `gap` while the connection
   held.

## 8. Decisions (2026-11-04, the user)

- The observer is not in the party (setup A): no party window; the "setup B" rows of section 5
  are out of these sessions.
- The records are more `bot.watch` lines in `caravand.log`, read directly by this repo's
  analysis script; no new file or format.
- The observer selects nothing (no 0x7045): a monster's HP is only its max HP minus the damage
  seen on it.
- No chat lines; no once-a-second `snap`; no raw hex payloads.
