# Rust + React re-implementation plan

Goal: re-implement OpenSteer steering behaviors in Rust, with a React page demonstrating each
behavior, faithful to the C++ and proven by C++ oracles. Each step is small and ends with a check.

- Stage 1, 2D (Phases 0-6): done.
- Stage 2, 3D (Phase 7): done so far: the empty arena (steps A, B, E) and the Silkroad world (steps
  C, D: terrain, floors, walls around Jangan). The 2D crates and demos stay alongside the 3D ones.
- Stage 3, the Training Field (Phase 8): a Silkroad training spot in miniature, built for the
  lurer; done through F6k (#46). What comes next: Status, "Next".

## Status (2026-11-03)

| Part | Steps | PRs | State |
|---|---|---|---|
| Phases 0-1 and step 8: skeleton, vehicle, One Turning, Seek and Flee, controls, legend | 1-8 | #1-#9 | done |
| Phases 2-5: 2D behaviors and composites | 9-20 | #10-#19 | done (steps 10-20 built as a batch) |
| Phase 6: Reynolds 1999 behaviors in 2D | 21-23 | #20-#22 | done |
| Phase 7 A/B/E: 3D core, 3D view, 3D demos | 24-37 | #23-#36 | done (steps 30-37 built as a batch) |
| Phase 7 C: the Silkroad world (navmesh copy, standing on the ground, terrain view) | C1-C3 | #38-#40 | done |
| Phase 7 D: walls (rectangle obstacle, wall query, the plaza with walls) | D1-D3 | #41-#43 | done |
| Phase 8: the Training Field, built for the lurer (field, combat, rarities, two lurers, planners, risk check, buffs) | F1-F6k | #46 | done (one branch, 35 commits; the user looked after most steps; a fresh-session observation is next) |

- The user looked at the 3D view (#26) and at the whole 3D batch and reported both "looking good"
  (2026-10-21); C2 (the walker on the ground) and C3 (the drawn terrain, "looks right") too; D3
  was approved for its PR (no explicit browser report). The 2D demos of steps 9-23 were never explicitly
  confirmed in the browser.
- Rhythm: one behavior at a time, held until the user understands it, and ask before each PR;
  batches (no holding, merge on green CI) only when the user asks for one.
- The Training Field was observed in a fresh session 2026-11-03 (Findings, "Observed in the
  browser"): no deaths, but the lurers stand at the centre two thirds of the time behind the
  strong cap, every "lost" was a packmate splashed at the disk's edge, and lurer B's status line
  was off the canvas. Agreed order from there (the user, 2026-11-03), one small branch and PR
  each:
  1. [done, #48] The status lines inside the map's square.
  2. [done, this branch] "Lost" counts a monster delivered when a dps's damage takes it over
     within the disk plus the blast's 2 (A 2/7/0, B 3/11/0 at seed 1; nothing else moved).
  3. [done, this branch] Strong cap default 1 -> 2, re-pinned, as the baseline for the
     obstacles' cost: the two minutes do not move (two strong arrive together and hold both
     lurers either way); at 180 s A 5 and B 5 trips, 24 kills, no deaths, at the centre
     93 / 113 s against 118 / 123 s.
  4. [done, #51 and this branch] Obstacles in the luring disk: 8 rocks by default, circles
     (#51) and rectangles at any angle (the numbers in Findings). The planner's detour cost
     only if the gap is large; no line-of-sight occlusion (Silkroad has none).
  5. The other Phase 8 Gaps worth tuning (the lurers' MP, lurer A's share, the load cap).
  6. [done, this branch] The risk check reads the party window bar by bar (the lowest member
     and the worst fall in 5 s) instead of the mean, which hid a dps at 11% (Findings).
  7. Next: one backlog slider (the dps work left in the disk, in seconds) in place of the load
     and strong caps, read within sight and decayed while out, the lurer's own chasers added.
  The 3D options offered 2026-10-21 stay for later:
  1. Stop bodies scraping walls: add the containment side probes (the design of steps 23 and 37) to the
     Silkroad walker, so walls it runs along or corners it rounds push it off (Findings: 299 of
     9600 plaza steps over a wall, closest 0.0019).
  2. Draw building floors (object navmesh cells) so the plaza walker does not float over pits.
  3. One-way ledges: drop off, never climb (now walls both ways; sro-polyanya keeps a blocked side).
  4. Other ground demos on the terrain (Pedestrians, Capture the Flag) with the same `World`.
- Tests: 323 Rust tests (13 in `silkroad-navmesh`, + 3 ignored full-data ones; 44 in
  `training_field`), 21 C++ oracles.
  Tests needing the game data skip without it (CI has none); with it, the Silkroad demo's tests
  take ~14 s in debug. 2D: `oracle_one_turning`, `oracle_seek_flee`,
  `oracle_wander`, `oracle_pursuit`, `oracle_obstacles`, `oracle_pathway`,
  `oracle_path_following`, `oracle_boids`, `oracle_neighbors` (in `steer-core`),
  `oracle_capture_the_flag`, `oracle_pedestrians`, `oracle_soccer` (in `steer-demos`). 3D (in
  `steer-core-3d`): `oracle_vehicle_3d`, `oracle_seek_flee_3d`, `oracle_wander_3d`,
  `oracle_pursuit_3d`, `oracle_obstacles_3d`, `oracle_path_following_3d`, `oracle_boids_3d`,
  `oracle_neighbors_3d`, `oracle_rectangle_3d`.
- Sidebar: 15 2D demos (the last is "Training Field"), then 15 3D demos (ids `3d-...`; the last is "3D Seek on Silkroad").

## Setup, mirrored from `~/Workspace/silkroad/caravan`

The new code lives in this repo next to the C++ reference (`crates/`, `packages/`, `apps/`).

```
Cargo.toml              workspace, members = ["crates/*"], [workspace.package] edition 2021,
                        rust-version 1.90, lints: unsafe_code forbid, clippy all warn
rust-toolchain.toml     stable + rustfmt, clippy
rustfmt.toml            edition = "2021"
.cargo/config.toml      [env] TS_RS_EXPORT_DIR = packages/core/src/contract (relative)
package.json            npm workspaces ["packages/*", "apps/*"]; build / dev / typecheck scripts
tsconfig.base.json      copied from caravan (ES2022, Bundler, strict, react-jsx)
.github/workflows/ci.yml  on push to main and PRs: fmt, clippy -D warnings, test, contract diff;
                        npm ci, typecheck, build. CI never builds the C++ oracles (their
                        output is pinned in the Rust tests).
scripts/dev.sh          builds steerd (debug), runs it + Vite; Ctrl-C (or either exiting) stops both

crates/steer-core       L1 pure 2D: Vec2, util, Vehicle + apply_steering_force, steer (seek, flee,
                        arrive, pursuit, evasion, wander, target speed, path following),
                        obstacle, pathway, proximity (grid), flock, neighbors, leader, wall,
                        containment. No tokio, no serde, no IO.
crates/steer-core-3d    L1 pure 3D: Vec3, util, Vehicle (stored forward/side/up, banking),
                        steer, obstacle (Obstacle trait: Sphere and Rectangle, local space),
                        pathway (tube), flock, neighbors, leader (cylinder), wall (Block, surface
                        following), containment (box).
crates/silkroad-navmesh L1 game data: caravan-navmesh frozen at caravan 0ad7987 (SOURCE.md), only
                        thiserror; plus our walls.rs (blocked edges as walls, merge_straight_runs).
crates/steer-demos      L2 one module per 2D demo: world setup, update(dt), click/action/param
                        input and its FrameView (serde + ts-rs). Demo enum, catalog, annotation
                        helpers, shared color meanings (`color`), `legend`, `controls`.
crates/steer-demos-3d   L2 one module per 3D demo -> Frame3View; Demo3 enum and catalog; reuses
                        steer-demos' color, legend, controls and view types for buttons/sliders;
                        `ground::Ground` lifts a 2D demo onto the 3D ground plane; `world::World`
                        (Jangan 3x3: surfaces, terrain grid, merged walls) for "3D Seek on
                        Silkroad" (`silkroad_seek`), which also sends a TerrainView.
crates/steerd           L3 daemon on 127.0.0.1:7400 (caravand uses 7300): axum 0.8 `/ws` only,
                        tokio, rmp-serde. contract.rs: Request { id, call }, Call (tag "method" /
                        content "params"), Reply (tag "type" / content "value"), HostFrame (tag "t"),
                        SceneFrame (tag "kind": flat | space). sim.rs: one Sim (2D or 3D demo +
                        running) owned by the sim task, fixed 1/60 s steps, commands in over mpsc,
                        latest frame out over a watch channel, the demo's ground
                        (HostFrame::Terrain) over a second watch channel when another demo is
                        chosen and on connect; catalog() = 2D then 3D.
                        origin.rs: caravan's LoopbackGuard (Host + Origin check, 403 otherwise).
                        Does NOT serve apps/web/dist yet: browse through Vite (scripts/dev.sh).

packages/core           @opensteer/core: MessagePack transport (@msgpack/msgpack) over a
                        WebSocket Link with reconnect, Host class with typed calls, Redux
                        Toolkit store whose reducers only copy what steerd sent, generated
                        contract types in src/contract/ (committed, never hand-written).
apps/web                @opensteer/web: React 19 + Vite + react-redux; Vite proxies /ws to
                        127.0.0.1:7400. three.js (as caravan), lazily loaded: `Scene` (2D,
                        orthographic top-down, world (x, y) at (x, 0, y)) or `Scene3D` (3D,
                        PerspectiveCamera 45 + OrbitControls, +y up, grid at y = 0, or the
                        demo's terrain as a lit mesh when the host sent one), chosen by the
                        frame's `kind`. Sidebar, controls, actions, sliders, legend overlay.
```

Dependency graph: `steer-core` <- `steer-demos` <- `steerd`; `steer-core-3d` and
`silkroad-navmesh` <- `steer-demos-3d` (which also uses `steer-demos`) <- `steerd`.

Rules carried over from caravan:
- A crate depends only on crates below it.
- **Frontend is dumb**: all steering math and annotations (velocity, force, predicted target,
  neighbor radius, legend, label text, camera focus) are computed in Rust into view models. The
  browser draws them and sends typed commands. Allowed client-side: visual interpolation between
  frames, turning a click into a ground point (input handling), and orienting a 3D body from the
  `forward`/`up` it is sent.
- Contract types are generated by `ts-rs` on `cargo test`; CI fails if they differ.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run typecheck`
  green before a commit. Errors: `thiserror` in libs, `anyhow` only in `steerd`. Logging: `tracing`.
- Pin exact npm versions (caravan pins exact versions).

Commands:
```
scripts/dev.sh                          # steerd + Vite together, open http://localhost:5173; Ctrl-C stops both
cargo test                              # all Rust tests, regenerates contract types
cargo test -p steer-core seek           # one test by name filter
cargo test -p steer-core --test oracle_seek_flee      # one 2D oracle comparison
cargo test -p steer-core-3d --test oracle_boids_3d    # one 3D oracle comparison
cargo test -p silkroad-navmesh          # navmesh + walls (data tests skip without the game data)
cargo run -p steerd                     # backend only, ws://127.0.0.1:7400/ws
npm install && npm run dev              # Vite on :5173, proxies /ws to steerd
npm run build && npm run typecheck
```

## Phase 0 - skeleton

1. [done] Workspace files above + empty `steer-core`, `steer-demos`, `steerd`. -> verify: `cargo test`, `cargo clippy -- -D warnings`.
2. [done] `steerd` serves `/ws`; contract with one call `demo.list`. `cargo test` writes TS types. -> verify: types appear in `packages/core/src/contract/`; a msgpack WebSocket client gets the demo list.
3. [done] `packages/core` transport + store, `apps/web` shows connection status and the demo list. -> verify: browser shows "connected" and the list; reconnects after a steerd restart.
4. [done] Sim loop: steerd ticks a fixed 60 Hz step; a three.js scene draws the frame. -> verify: 60.5 frames/s measured; moving dot.

## Phase 1 - vehicle and integrator (`src/SimpleVehicle.cpp`)

5. [done] `Vec2` (`length`, `normalize`, `truncate_length`, `dot`, `perp`, parallel/perpendicular components) + `util` (`interpolate`, `blend_into_accumulator`, `limit_max_deviation_angle`). `rotate`/`distance` come when a behavior needs them. -> verify: unit tests.
6. [done] `Vehicle { position, forward, speed, mass, radius, max_force, max_speed, smoothed_acceleration }` (C++ keeps forward + speed, velocity = forward * speed) and `apply_steering_force(force, dt)` ported from `applySteeringForce` incl. `adjustRawSteeringForce`. Curvature and smoothed position (annotation-only) not ported. `up` and `regenerate_basis` were added in step 20 (see Findings). -> verify: property tests + `tests/oracle_one_turning.rs` matches the C++ `SimpleVehicle` (`oracle/one_turning.cpp`) to 1e-4 over 600 steps; a real mutation (clamp floor 0.15 -> 0.2) fails at step 60.
7. [done] Demo "One Turning Away" (`plugins/OneTurning.cpp`): constant force (-2, -3), trail, velocity/acceleration lines, the push drawn as a 4.5-unit direction under them, speed and start labels, camera follows the vehicle (Rust sends `focus`). -> verify: numeric match via the oracle; the C++ OpenSteerDemo app itself was never built or run, so there is no side-by-side visual comparison.

## Phase 2 - single-target behaviors (`include/OpenSteer/SteerLibrary.h`)

8. [done] Seek and Flee (port `xxxsteerForSeek/Flee`; `tests/oracle_seek_flee.rs` vs `oracle/seek_flee.cpp`, 1e-4 over 300 steps). Demo "Seek and Flee": click sets the target, Seek/Flee buttons, want (green) and push (red) drawn at true length (max_speed 3, so pink is also true length and red = green - pink closes a triangle).
9. [done] Arrive (Reynolds 1999, not in OpenSteer, so no oracle): `steer::arrive(v, target, slowing_radius)`, desired speed `max_speed * min(d / r, 1)`. Plus OpenSteer's `applyBrakingForce` as `Vehicle::apply_braking_force` (CaptureTheFlag, Soccer use it). Third mode in Seek and Flee, a Brake toggle (brakes inside the radius), sliders for slowing radius (1-15, 9) and braking rate (0.25-4, 0.75), radius circle. Tests pin the Findings cases.
10. [done] Wander (`steer::wander`, `util::scalar_random_walk`) and `steer::target_speed`. Random source injected as `FnMut() -> f32`; `oracle/wander.cpp` builds every file with `-Drand=oracle_rand` (an LCG) and `tests/oracle_wander.rs` replays it (two draws per call: side, then the unused up), 1e-4 over 600 steps; a 12 -> 11 walk-speed mutation fails at step 60. Only `WanderSide` is ported in 2D (up has no effect on the ground). Demo "Wander": `MpWanderer` (max_force 5, max_speed 3, push = forward + 3 * wander), buttons Forward push / Target speed, sliders wander weight and target speed, seeded `fastrand` so reset repeats the walk. Demos share `controls::{button, slider, Range}`.
11. [done] Pursuit and Evasion: `steer::intercept` (the 9-case table, returns target, factor and case name), `steer::pursuit` (plain seek of it), `steer::evasion` (plain flee; the C++ does not compile, ported as intended), `Vehicle::predict_future_position`. `oracle/pursuit.cpp` (pursuer vs the LCG wanderer; evader vs pursuer with a repaired copy of `steerForEvasion`), 1e-4 over 600 steps; mutations (factor 4 -> 3.5, no evasion cap) fail. Demo "Pursuit and Evasion": `MpPlugIn` (wanderer + 1-30 pursuers on a 20-30 ring, restart on touch, slider), aims drawn yellow instead of the C++ 9 case colors (red/green are reserved), the nearest pursuer labeled with its case and factor; mode "Pursuit vs evasion" (one hunter, evader max speed 2).

## Phase 3 - environment

12. [done] `steer_core::obstacle`: `Circle` (`SphereObstacle` on the ground, `SeenFrom` outside/inside/both), `intersect_path`, `nearest_intersection`, `avoid_obstacles` (`steerToAvoidObstacles`). `oracle/obstacles.cpp`: LCG wanderer in a container with three rocks, avoidance wins when nonzero (Pedestrian's priority); 1e-4 over 1800 steps incl. the count of avoiding steps (222). Mutations: no vehicle-radius widening fails at step 360; "inside a solid obstacle" distance 0 -> 1 was missed by the oracle, now caught by a unit test. Demo "Obstacle Avoidance": the oracle scene, click adds a rock, "Remove added rocks", slider min time to collision, look-ahead corridor and hit point (teal), obstacles brown.
13. [done] `steer_core::pathway::Pathway` (`PolylineSegmentedPathwaySingleRadius`): `map_point` (on-path point, tangent, outside, path distance: `mapPointToPath` + `mapPointToPathDistance`), `point_at` (`mapPathDistanceToPoint`), `length`, open or cyclic. Verified by a C++ oracle instead of the cppunit tests (those are not wired up): `oracle/pathway.cpp` / `tests/oracle_pathway.rs`, 12 points and 9 distances on an open and a cyclic path, incl. corner ties and out-of-range distances; mutations (`<` -> `<=` tie, no cyclic wrap) fail.
14. [done] `steer::follow_path` / `stay_on_path` (+ `*_target` for annotation; plain seek). `oracle/path_following.cpp`: the eight's path (y set to 0), one pedestrian `forward + 0.5 * path force`, directed with endpoint flips and stay-on-path, 1e-4 over 3600 steps; mutations (no right-way check; stay threshold 0 -> 0.5) fail. Demo "Path Following": 1-50 pedestrians (slider) on the eight with `Pedestrian::reset`'s random start, toggles "Follow in a direction" and "Wander" (both on, as the plug-in), pedestrian 0 shows its 3 s look-ahead, seek target and push.

## Phase 4 - groups

15. [done] `steer_core::proximity`: `brute_force` and `Grid` (square bins, `move_point` = `updateForNewPosition`), both return sorted indices strictly inside the radius. Tests: grid equals brute force for 200 queries at 4 radii and after 100 moves; a too-small cell range fails.
16. [done] `steer_core::flock`: `in_neighborhood`, `separation`, `alignment`, `cohesion` over an iterator of the other vehicles. `oracle/boids.cpp`: 12 boids with the plug-in's constants, sequential update, circular wrap-around, all 12 pinned every 100 steps for 600 steps; built with `-ffp-contract=off`. Mutations (`powi` back, 1/d separation, min neighborhood 2 radii) fail. Demo "Boids": 10-300 boids (100) in a radius-50 wrap-around circle, sliders for count and the three weights (12/8/8), grid kept current during the pass (a test: bit-identical to the whole flock over 600 steps), boid 0 shows its flockmates and push. Each frame carries `view_size` (the camera's width in world units).
17. [done] `steer_core::neighbors`: `nearest_approach_time`, `approach_positions`, `avoid_close_neighbors`, `threat` (earliest predicted approach within 2 radii, and the side), `avoid_neighbors`. `oracle/neighbors.cpp`: 8 pedestrians crossing a circle (Pedestrian's priority: forward + 10 * avoidance, else forward + seek), all pinned every 150 steps over 900 incl. the avoiding count; mutations (danger 2 -> 1.5 radii, no close-neighbor push) fail. Demo "Pedestrians Crossing": 2-30 on a circle walk to the opposite point and back, toggle "Avoid neighbors", count of overlapping steps, pedestrian 0's predicted nearest approach and push.

## Phase 5 - composite demos

18. [done] Demo "Capture the Flag" (`steer_demos::CaptureTheFlag`): the plug-in's seeker (`clearPathToGoal`, `XXXsteerToEvadeAllDefenders`, look-ahead 0.9/2 s, `limitMaxDeviationAngle` 0.707), 4 pursuing enemies, 40 random rocks (add/remove buttons), braking after a tag or goal and a restart 4 s later. `oracle/capture_the_flag.cpp` copies the plug-in logic with a fixed layout; `crates/steer-demos/tests/oracle_capture_the_flag.rs` pins two rounds (seeker evading for 20 s; seeker tagged at ~11 s, then braking) every 60 steps; mutations (evade behind-weight, corridor width, enemy braking) fail.
19. [done] Demo "Pedestrians" (`steer_demos::Pedestrians`): `Pedestrian.cpp`'s path and two rocks, 1-200 pedestrians (100), `determineCombinedSteering` (obstacle avoidance 6 s, else neighbor avoidance x 10, else wander + 0.5 x directed path following; both avoidances skipped 10% of steps), neighbors from the grid within 12. `step(dt, random01)` takes the random source. `oracle/pedestrians.cpp` (fixed layout of 10, LCG draws in the plug-in's order) pinned every 180 steps over 1800 incl. the avoidance counts; mutations (leak 0.2, one wander draw) fail. Not ported: the plug-in's tilted `RectangleObstacle` (see Gaps).
20. [done] Demo "Soccer" (`steer_demos::Soccer`, "Michael's Simple Soccer"): 8 v 8 on their spots, kick on touch, neighbor avoidance (1 s) against the players created earlier, seek the ball / get behind it / go home, a braking, bouncing ball, goals restart it. Deterministic (the plug-in's random draws are overwritten). `oracle/soccer.cpp` + `crates/steer-demos/tests/oracle_soccer.rs`: a minute, ball, score and two players every 120 steps; bit-identical traces over 3600 steps after the `side` fix (Findings); mutations (exact-perp side, avoid everyone, no ball braking) fail.

## Phase 6 - Reynolds 1999 behaviors not in OpenSteer, 2D (no C++ oracle)

Built from the ported, oracle-checked pieces; tests pin hand-computed cases and measured facts.

21. [done] Demo "Leader Following" (`steer_core::leader`, `steer_demos::LeaderFollowing`): followers `arrive` (slowing radius 3) at a point `behind` the leader plus weighted `separation` (2 units, rear cone excluded) from the others and the leader; inside the leader's path (4 ahead, leader radius + a follower diameter to each side) they instead want max speed sideways out of it. Leader: the Wander demo's push (max speed 1.5), followers max speed 3. Sliders: followers 1-20 (8), distance behind 1-8 (3), separation weight 0-5 (2). Mutation (step aside the wrong way) fails a core and a demo test.
22. [done] Demo "Wall Following" (`steer_core::wall`, `steer_demos::WallFollowing`): `Wall` is a closed counter-clockwise polygon (`edges`, `distance`, `contains`, `offset` with miter corners: edges stay exactly the offset away, a right-angle outer corner is offset x sqrt 2); `wall_path` makes the offset outline a cyclic `Pathway` (reversed for clockwise, so direction is always +1 and the cyclic `fmod` quirk at negative distances never applies). Wall following is then the ported `follow_path`: three walkers (max speed 2, max force 8) steer with `forward + steerToFollowPath` around an L-shaped building, band radius 0.5. Buttons: counter-clockwise / clockwise; sliders: offset 0.5-6 (2), look-ahead 0.25-3 s (1). Mutation (inward normal) fails 3 core tests and the demo measurement.
23. [done] Demo "Containment" (`steer_core::containment`, `steer_demos::Containment`): `Wall::contains` (even-odd ray crossing); three probes `radius + look_ahead * speed` out (ahead, 45 degrees to each side); while the ahead probe is outside, a `max_force` push along `side` toward the side probe with more room (signed distance to the wall, +side on a tie). Five wanderers (the Obstacle Avoidance demo's wander, max speed 3, max force 5) in an L-shaped room; while the push is on they also brake (`apply_braking_force`). Sliders: wanderers 1-20 (5), look-ahead 0.25-3 s (1), brake while containing 0-4 (4). Label: escapes, wanderer-steps outside, furthest out. Four designs were tried (Findings). Mutation (turn toward less room) fails 4 core tests and both demo measurements.

## Phase 7 - 3D (stage 2)

Decided with the user 2026-10-21: keep the 2D crates and demos; add 3D alongside. Arena first,
then the Silkroad world. The four 1999 behaviors get full 3D versions. The world's navmesh code is
copied into this repo (not a path or git dependency), so CI builds it; tests needing the game data
(`~/Workspace/silkroad/Data`, 2.9 GB, of which `navmesh/` is 577 MB; `CARAVAN_GAME_DATA` overrides
the path) skip when it is missing, as caravan's do.

Research (2026-10-21, caravan and sro-polyanya read-only):
- The Silkroad world is 2.5D: per 1920-unit region (region id = `x | z << 8`) a 97x97 height grid
  over 96x96 tiles of 20, plus object meshes (buildings, bridges) with stacked floors. Y is up.
  Characters walk on surfaces; nothing flies. Caravan's three.js scene uses `(x, height, -wire_y)`
  (north is -z); conversions in `caravan/crates/caravan-protocol/src/coords.rs`.
- `caravan/crates/caravan-navmesh` (Rust, depends only on `thiserror`, `publish = false`):
  `DataDir::folder(path)`, `Navmesh::load(&data, &allow_regions)`, `Region::height_at(x, z)`,
  `ObjectResource::height(x, z, area)`, `heights::surface_heights(navmesh, frame, x, z)`;
  region and object edges carry flags (blocked, one-way ledge, bridge). No point location, nearest
  point on mesh, wall queries or path finding (caravan deferred its wall model, decision D32).
  Tests: Jangan plaza heights (`navmesh.rs`), skipped without caravan's default `../Data` (our
  copy defaults to `~/Workspace/silkroad/Data`).
- `sro-polyanya` (C++, FastAPI on 127.0.0.1:5577): Polyanya paths (`/path`) and line of sight
  (`/los`); long queries take ~20 s and one search runs at a time; no height, point-location or
  wall query in its API. `/geometry` dumps terrain positions and object wall edges.
- Caravan's renderer: `PerspectiveCamera(45)` + `OrbitControls`, terrain meshes built from
  `~/.caravan/map/<region>/` (`region.json|bin`, `terrain.ktx2`, `nav.bin`; 4025 regions, built by
  `caravan-import`), served by caravand under `/map`.

Steps A, B, E (done; every 3D oracle builds with `-ffp-contract=off` and checks all 13 state
values - position, forward, side, up, speed - at step 1 and at regular steps):
24. [done, A] `steer_core_3d::{Vec3, Vehicle}`: `Vec3` (cross, truncate, components, `set_y_to_zero`), the vehicle with stored `forward`/`side`/`up` (`regenerate_basis` = `regenerateOrthonormalBasisUF`), `apply_steering_force`, `apply_braking_force`, `banking` (`regenerateLocalSpaceForBanking`). `oracle/vehicle_3d.cpp`: a constant climbing push and a banking circle, 600 steps. Mutations (left-handed side, banking blend 2 instead of 3) fail.
25. [done, A] `steer::{seek, flee}`. `oracle/seek_flee_3d.cpp`: from rest toward (6, 2, -4) and away from (1, -1, 1), 300 steps. Mutation (uncapped want) fails seek. The 3D oracle tests share `crates/steer-core-3d/tests/common/mod.rs` (state order, 1e-4 `check`, `OracleRand`).
26. [done, A] `steer::{wander, WanderState, target_speed}`: both walks (`WanderSide`, then `WanderUp`: two draws per call). `oracle/wander_3d.cpp`: MultiplePursuit's wanderer without `setYtoZero`, forward push and target speed 1.5, 600 steps, LCG replayed. Mutation (up drawn before side) fails both.
27. [done, B; viewed by the user] Crate `steer-demos-3d` (`Frame3View` and friends, `Demo3`, catalog after the 2D demos); steerd sends `SceneFrame::{Flat, Space}`; web `Scene3D` (PerspectiveCamera 45, OrbitControls without pan, grid at y = 0, vehicles as a triangle with a mast along `up`, labels, height stems). Demos "3D One Turning" (climbing turn / banking circle, both reproduce `oracle_vehicle_3d`), "3D Seek and Flee" (reproduces `oracle_seek_flee_3d`; Move target hops between four heights), "3D Wander" (forward or target speed, wander weight, banking toggle). Step B was brought forward so each later 3D behavior arrived with its demo.
28. [done, A] `steer::{intercept, Intercept, pursuit, evasion}`; demo "3D Pursuit and Evasion" (pursuers on a 20-30 shell with random 3D headings, catch counter). `oracle/pursuit_3d.cpp`: pursuer vs the 3D wanderer, evader vs hunter (both vehicles checked), plus every cell of the 3x3 time-factor table (the runs reach only 5 of 9; a "behind, parallel" change passed them and fails the cells test). Mutation ("ahead, parallel" 4 -> 2) fails all three.
29. [done, A] `obstacle::{Sphere, SeenFrom, intersect_path, nearest_intersection, avoid_obstacles}` and `Vehicle::localize_position`: the intersection is computed in the vehicle's local space as the C++ (the 2D port uses world offsets). Demo "3D Obstacle Avoidance" (container sphere 15, three rocks at different heights, Add a rock / Remove added rocks, min time slider; spheres as three great circles, the look-ahead as four lines). `oracle/obstacles_3d.cpp`: 1800 steps incl. the avoiding count. Mutation (no vehicle radius in the widening) fails; world offsets instead of local space pass (bit-level only).
30. [done, A] `pathway::Pathway` (the radius is a tube), `steer::{stay_on_path(_target), follow_path(_target)}`. Demo "3D Path Following" on the figure eight raised into a roller coaster (heights 4 sin(i * 45 degrees)), tube drawn as rings. `oracle/path_following_3d.cpp`: directed and stay on path, 3600 steps, not flattened. Mutation (flat distance: a band, not a tube) fails both.
31. [done, A] `flock::{in_neighborhood, separation, alignment, cohesion}`, `Vec3::spherical_wrap_around`, `util::{random_vector_in_unit_sphere, random_unit_vector}`. Demo "3D Boids": the plug-in as OpenSteer has it (banking, world sphere 50, 10-150 boids, weight sliders). `oracle/boids_3d.cpp`: 30 boids from the plug-in's random reset (LCG replayed), 600 steps, every 7th boid and the wrap count. Mutation (banking off) fails.
32. [done, A] `neighbors::{nearest_approach_time, approach_positions, avoid_close_neighbors, threat, Threat, avoid_neighbors}` (sideways is the stored `side`). Demo "3D Crossing": walkers on a Fibonacci sphere walk to the opposite point and back. `oracle/neighbors_3d.cpp`: 12 walkers, 900 steps, avoiding count. Mutation (the slower one gives way) fails.
33. [done, A] The composites in the 3D view: "3D Capture the Flag (ground)", "3D Pedestrians (ground)", "3D Soccer (ground)" (`steer_demos_3d::ground::Ground`). The plug-ins flatten every force (`setYtoZero`), so they are not re-ported: the oracle-checked 2D runs are lifted onto the ground plane ((x, y) -> (x, 0.02, y) for annotations, up straight up). A test checks the 3D view is the 2D run lifted. These are the first candidates for Silkroad terrain (step C).
34. [done, E] `steer::arrive` in 3D; "3D Seek and Flee" gains Arrive (slowing sphere), Brake and the two sliders, as in 2D.
35. [done, E] `leader` (the leader's path is a cylinder; stepping aside goes straight out from its axis, along the leader's side when on it). Demo "3D Leader Following" with a 3D wandering leader.
36. [done, E] `wall::{Block, SurfacePoint, follow_surface(_target)}`: wall following as the paper describes it (predict, nearest surface point, out along the normal by the gap, seek), since a 3D surface has no closed outline to follow. Demo "3D Wall Following": three walkers hug a floating block (`forward + follow_surface`), gap and look-ahead sliders. Mutation (normal flipped: the target inside the block) fails.
37. [done, E] `containment::{probes, contain, room}`: the 2D design in 3D, five probes (ahead, and 45 degrees toward +-side and +-up), push toward the roomiest side probe while the ahead probe is outside. Demo "3D Containment": five wanderers in a 24 x 12 x 16 box room, braking while containing; wanderers, look-ahead and brake sliders. Mutation (turn toward the least room) fails 2 core tests and the demo measurement.

Steps C, D (done 2026-10-21, one small step and PR each). Units: the demo is the game world scaled
by 1/10 (a region is 192 demo units); game units are used inside `World` and `silkroad-navmesh`.
- C. The world:
  - C1. [done, #38] `crates/silkroad-navmesh`: caravan-navmesh at `0ad7987`, source note in
    `SOURCE.md` (renamed; test data defaults to `~/Workspace/silkroad/Data`, `CARAVAN_GAME_DATA`
    overrides). 9 tests pass with the data (Jangan heights and objects match sro-polyanya), the 5
    data tests skip without it; the 2 ignored full-data tests (every region file, every object)
    pass in release. No steering yet.
  - C2. [done, #39] `steer_demos_3d::world::World` loads Jangan and its 8
    neighbors (< 0.1 s in debug) and lists the walkable surfaces over (x, z)
    (`surface_heights`: terrain where its tile is walkable, then object floors) in Jangan's
    frame (game units). Demo "3D Seek on Silkroad" (`3d-silkroad-seek`): the world scaled by
    1/10, the 3D Seek vehicle (force 1, speed 3) seeks the target at its own height (a level
    seek), then stands on the highest surface at most a step (0.5) above its feet, else the
    nearest. Two places: "Hill" (four targets around a hill south of Jangan) and "Plaza" (from
    outside the west gate, over the gate, down the ramp, then plaza positions from caravan's
    height test). Without the game data the ground is flat. Tests: the xz track equals the flat
    run bit for bit at both places; hill highest ground per leg -3.40 / -0.02 / -4.40 / 0.34;
    plaza: gate 0.097, then -3.26 on every leg, no step down more than 0.5. Mutations (seek the
    target's real height; skip the height; nearest surface instead of step-up; step 0; mirrored
    lookup) fail. ~0.24 ms per step on the plaza in debug. (D3 later changed the walker's limits,
    the Plaza targets and the pinned numbers; see D3.)
  - C3. [done, #40; viewed by the user] The terrain reaches the browser once per demo, not every
    frame: `World::terrain_grid(step)` (the regions' height points every `step`-th, shared edges
    taken once; neighbors' edge heights are equal), `TerrainView` (origin, spacing, size, heights,
    color), `Demo3::terrain()`; steerd sends `HostFrame::Terrain { demo, terrain }` on its own
    `watch` channel when another demo is chosen and on connect. The store keeps it with its demo
    id; `Scene3D` draws it (`Terrain3`: indexed mesh, flat-shaded Lambert under a hemisphere and
    a directional light, pushed back with polygon offset so lines on the ground show) when it
    belongs to the shown demo, and hides the y = 0 grid. "3D Seek on Silkroad" sends 73 x 73
    points 8 apart (every 80 game units) over the 3x3 regions; building floors are not drawn.
    Tests: the grid's points are the height points (mutation: x and z swapped fails); the view is
    the walked terrain scaled; the terrain message names the demo.
- D. Walls from the navmesh's blocked edges. Decided with the user 2026-10-21: walls are OpenSteer
  `RectangleObstacle`s (vertical rectangles, avoided from both sides) with the ported obstacle
  avoidance; a walker reacts only to walls of its own floor (each wall keeps its floor height;
  within the step height); the wall query is a new `walls` module in `silkroad-navmesh`; the
  first demo is the Plaza of "3D Seek on Silkroad". Facts (Jangan): 23 blocked terrain edges,
  all on tile lines (2640 game units; D2 found 15 of them on the region border, skipped, and 8
  fencing the two disabled tiles); 2 of 9216 tiles disabled; 62 objects with 664 blocked
  outline edges and 77 blocked inline edges, so in town the walls are object outlines.
  Proposed order, one small step at a time:
  - D1. [done, #41] `obstacle::{Rectangle, Obstacle}`: OpenSteer's
    `RectangleObstacle` (`PlaneObstacle::findIntersectionWithVehiclePath` with
    `xyPointInsideShape`, in the rectangle's local space); `nearest_intersection` and
    `avoid_obstacles` take any `Obstacle` (spheres unchanged). `oracle/rectangle_3d.cpp`: 192
    single poses (an upright 6x2 wall seen from outside, inside, both; Pedestrian.cpp's tilted 7x7
    rectangle) and a seek back and forth across both, 1800 steps. Mutations (no radius widening,
    normal from the wrong side, no "toward the edge" hint) fail both oracle tests.
  - D2. [done, #42] `silkroad_navmesh::walls::{walls, terrain_walls,
    object_walls, Wall}`: blocked edges as segments (a, b as (x, height, z)) in one region's
    frame, by sro-polyanya's rules (read with a helper; caravan's debug overlay agrees):
    terrain internal edges with a block bit, skipping those on the region border; object outline
    edges 0x03 / bridge rim 0x10 / siege 0x80; object inline edges with a block bit. Simplified:
    one-way ledges are walls both ways; outline 0x08 (stitched to another object) is open;
    region-border walls from closed tiles are not built. Jangan: 8 terrain walls (two closed
    tiles), 762 object walls (664 + 21 + 77); 3x3: 547 objects. The plaza hole along z = 854 is
    fenced by walls at x 878.9 and 1072.3, height -32.6. Mutations (no border skip, no bridge
    rims, a wall from one vertex twice) fail.
  - D3. [done, #43] "3D Seek on Silkroad" with walls. `World` keeps the walls,
    straight runs merged (`walls::merge_straight_runs`: shared endpoint, within 5 degrees in 3D;
    6481 -> 5887). Each step, the walls within 12.5 whose height range holds the walker's (give
    or take the 0.5 step) become upright rectangles (8 tall, seen from both sides); avoidance
    (look-ahead 6 s, Pedestrian.cpp's `oTime`) wins when nonzero, the push flattened; otherwise
    the level seek. The walker has Pedestrian.cpp's limits (force 8, speed 2). Walls within 20 of
    the walker's floor are drawn (`color::OBSTACLE`). The Plaza's second target is straight
    across the hole. Tests (four 40 s legs): the track equals the flat seek's until the first
    avoiding step (hill 5257, plaza 2655); hill: highest ground per leg, avoiding only at a
    sunken slab (124 steps), body over a wall 40 steps, closest 0.363; plaza: never drops
    (-3.261 on legs 2-4), avoids the hole 565 steps and reaches the far side, body over a wall
    299 steps, closest 0.0019 (a corner of the hole). Mutations (unmerged walls, walls of every
    height, push not flattened) fail. ~0.6-0.9 ms per step in debug; the demo tests take ~14 s.

## Phase 8 - Silkroad training field, built for the lurer (2D, no C++ oracle)

Chosen by the user 2026-11-02 over the 3D options. A miniature of a Silkroad training spot,
only what the lurer needs: a team trains round one centre while monsters spawn at nests on a
disk map. Built from the oracle-checked pieces like Phase 6 (hand-computed unit tests, measured
facts pinned, one mutation per step) on one branch with small commits, merged as #46 when the
user called the milestone (2026-11-03).

### As built (after #46)

One module, `crates/steer-demos/src/training_field.rs` (the 44 tests in the same file), demo
id `training-field`, the last 2D demo. Units: 1 = 1 m; Silkroad's disks of 20 and 60 are 10
and 50 here. Numbers are constants at the top of the module and `Settings` sliders.

- The field: a disk map of `MAP_RADIUS` 60 (nests an `IDLE_RADIUS` 6 inside its edge, never in
  the dps disk); one centre; the team of eight (the game's party limit): lurers A and B at
  (-1.5, 0) and (1.5, 0), four dps 4 out at the diagonals, the HP buffer at (0, 2.5), the MP
  buffer at (0, -2.5). Radii: dps 10, buffers 5, lurers 50 (`Role::radius`). Players always
  run (5); a roaming player seeks random spots of its disk (`roam`, containment as the guard).
  No collisions between bodies (the user).
- Rocks (`Rocks`): `DEFAULT_OBSTACLES` 8 obstacles (slider 0-20) in the luring disk, each a
  circle or a rectangle at any angle by a coin, a circle's radius and a rectangle's half sides
  `ROCK_MIN` 1.5 to `ROCK_MAX` 5, drawn by `place_rocks` from their own rng (seed 2) after the
  nests, so the monsters' draws stay; clear (by bounding radius) of the dps disk by 1, of a
  nest's idle disk by a body, of each other by 3; the slider only adds or removes rocks, and a
  later nest keeps that room. Lurers Out and Back, chasing monsters outside the dps disk and
  monsters walking home steer round them `AVOID_TIME` 2 s ahead, which wins over the seek:
  OpenSteer's `avoid_obstacles` for the circles, else `steer_core::containment::avoid` for the
  rectangles widened by the body's radius (the mirror of `contain`: a push while the line to
  the ahead probe meets the block, toward the side probe with more room outside); the dps and
  buffers never meet one. Drawn in `color::OBSTACLE`.
- Monsters: `DEFAULT_MONSTERS` 15 (slider 5-60); rarity by `SPAWN_MIX` 80 / 12 / 3 / 4 / 1 / 0
  percent (normal, champion, giant, party normal, party champion, party giant); HP x1 / x2 / x20
  / x10 / x20 / x200 (the game client's, read by caravan), damage x1 / x1.5 / x3 / x2 / x3 / x6
  (ours), bodies x1 / x1.25 / x2; strong = HP x10 or more. Idle: a gait drawn every 2-6 s
  (standing, walking 1.5, running at the monster speed slider, default 6) within 6 of the nest;
  whole again at home with nobody after it. Alarm disk 6; it chases whoever dealt it the most
  damage (`threat`), else the nearest player in its alarm disk, kept within `LOS_REACH` 35 and
  forgotten beyond; a bite of `BITE` 3 times the damage multiplier every 1.5 s inside the bite
  reach slider (2); dead at 0 HP, reborn at its nest after 3 s.
- Players: HP and MP 100, +1 of each a second; dead at 0, back at the centre after 5 s with full
  bars, forgotten by every monster. Dps: the nearest monster inside the dps disk, a run into
  reach 8, a blast every 1.5 s for 6 MP: 10 off the five nearest within 2 of the target, each
  hit +10 threat. Buffers (within reach 8, one cast a second, the member with the lowest bar
  under 90% and no such buff running, themselves included): HP buff +4/s for 10 s (4 MP), MP
  buff +3/s for 10 s (2 MP); the HP buffer first tries a heal (+40 on a member under 50%, 15 MP,
  3 s), a group heal (+20 to everyone in reach when the party's mean HP is under 70%, 25 MP,
  10 s), a guard (bites x0.8 for 20 s, 10 MP, 5 s, on the attacked member with the lowest HP).
- The lurer's kit: tag (reach 8, 1 damage, 2 MP, 1 s), blast (radius 3 round a monster within 8,
  6 MP, 4 s), pierce (12 long, 1.5 either side, toward a monster, 8 MP, 6 s); every hit a tag
  (the lurer on the threat table); an area skill only when it tags two or more untagged
  monsters, the pierce on a tie.
- The lurer's trip (`Lure`, `LureStage`, `lure_step`). Centre: stand until HP >= `LEAVE_HP` 90%,
  the dps disk within the load cap (3; 0 off) and under the strong cap (2; 0 off), the party not
  halted (every bar >= 50% and no member fell more than 30 in 5 s) and, for B, no earlier lurer in sight
  within 15 of the centre that is healthy or moving (at most 5 s). Out: plan every 1 s over the
  8 nearest candidates (untagged by any lurer; mine, by the nearer visible lurer with ties to A;
  within my sight 30; reachable from inside the disk; strong ones only while the disk holds
  fewer strong than the cap, every bar >= 70% and no member fell more than 15 in 5 s): the cast
  planner (default; a stop is a monster to get within a skill's reach of; the shortest run that
  tags N, the fewest casts on a tie) or the stops planner (button); run to the first stop; back
  when N (3) chase, HP < 40%, or nothing more is in sight after a tag; with nothing in sight and
  nothing tagged, scout random spots of the band 30..47 away from where the other lurer was last
  seen (10 s memory). Back: home within 1; the session's time and HP recorded.
- What a lurer knows (`observe`): monsters and the other lurer within 30 of itself, every
  member's HP (the party window), which player a monster chases, its own memory; never the
  other lurer's stage or plan, never a position beyond 30.
- The view: bodies by role color; HP/MP bars on dark tracks; every reach as circles
  (`CircleView`: ranged 8 and alarm 6 amber, melee violet, LOS 35 lilac on chasing monsters
  outside the dps disk, each lurer's sight 30); the planned tour or scouting spot as a gray
  polyline; attack lines and blasts; buff and guard rings; rarity labels; three status lines
  (the team; lurer A; lurer B); the legend behind a button. Sliders: monsters, N, monster
  speed, bite damage, bite reach, load cap, strong cap; buttons: plan casts / plan stops.
- Tests (44): hand-computed cases per piece, the pinned two-minute run (seed 1, 7,200 steps,
  `trips_deliver_monsters_to_the_dps_disk`, its history in the comment), one mutation per step
  that failed. Helpers: `quiet_field` (five normal monsters in a far corner, outside the map),
  `field_with_a_monster_at`, `risky_field` (a giant and a normal 20 out on A's side),
  `step_with_parked_lurer` (A put back at a spot after every step), `run_tracking_a` (tags by
  monster id over a run), `measure` and `Measured` (margins, speeds, time in the buff disk).

### Next step: obstacles in the luring disk (the user, 2026-11-03; circles #51 and rectangles done)

Random obstacles of varying shape and size inside the luring disk. Settled with the user
2026-11-03 (after the browser observation): circles first, a slider 0..20 (default 8), radius
1.5..5, drawn from a second rng (seed 2) after the nests so the monsters' draws stay today's,
clear of the dps disk by 1, of a nest's idle disk by a body, of each other by 3; avoidance first
(look-ahead 2 s) for the lurers Out and Back and for monsters outside the dps disk; rectangles
next; no line-of-sight occlusion (Silkroad has none); a planner detour cost only if the gap is
large. The original notes:
- Shapes: circles first (`steer_core::obstacle::Circle`, oracle-checked, the Obstacles demo's
  `avoid_obstacles` and its "Add a rock" placement with a clearance rule), then polygons through
  `steer_core::wall::Wall` (rectangles at any angle; `Wall::contains`, `distance`), each drawn
  in `color::OBSTACLE` (circles as `CircleView`, polygons as lines).
- Placement: random inside the luring disk, outside the dps disk, clear of nests, of each other
  and of the centre's paths home by a margin; a slider for the count; a seed so runs stay pinned.
- Who avoids: the lurers on their way out and home and the monsters chasing (avoidance first,
  as Pedestrians and Capture the Flag do); the dps and buffers stay inside the dps disk, which
  holds no obstacles.
- Sight: an obstacle blocks line of sight, for the lurer's 30 (a monster behind a rock is unseen)
  and for a chasing monster's 35 (a lurer that rounds a rock may be lost); a segment-circle or
  segment-polygon test; the alarm disk too, or not (ask).
- The planners cost straight lines; with obstacles the run is longer. Start by keeping
  straight-line planning with avoidance at execution and measure the gap (walked per delivered,
  session time), then decide whether the planner needs a detour cost.
- Order proposed: circles with avoidance and placement; polygons; sight occlusion; the planner.
  Each with the pinned two-minute run, so the obstacles' cost is visible against today's numbers.

### Build log (research, requirements, steps F1-F6k)

Research (caravan read-only, D105/D107, its `hunt::Lure`, status.md § Gaps "Luring"; RSBot's
`Botbases/RSBot.Lure`; the watched PhBot lures, world units = wire / 10): a party kills at a
training disk (20) while the lurer runs into the band up to the luring disk (60), hits each
monster once so it chases, leads them back and waits ~2 s (PhBot's centre delay; never above 5 s
in the small disk). The lurer never kills. Watched: trips to ~40 every ~9 s; tags a median 32-38
out; 145 of 229 monsters attacked the lurer first, a median 2.5 s after the hit; chases end a
median 7.4 from the lurer; half the lured reach the centre within 6 s; 5 of 111 chasers lost;
monsters run 16.5 a second with melee reach 3; a bow reaches 18. RSBot's lure walks to random
points at least radius / 1.5 away, attacks any monster not already attacking and cancels the
swing. Caravan's lure (a Centre / Out / Back walk state machine, no steering) was judged by the
user "too close to the training disk, back too soon and too often, wandering past monsters that
wait outside"; this demo exists to compare lurer strategies with measured metrics.

Requirements (the user 2026-11-02; defaults are ours, changed on their word):
- The map: a 160 x 120 rectangle round the centre; monsters spawn at random inside it, outside
  the dps disk, kept at a count (slider 5-60, default 30), a new one after each death.
- The team, one centre, a radius per role: three dps (10), an HP buffer and an MP buffer (5),
  our lurer (50). Bodies: lurer green, teammates blue, monsters red (muted tones; annotation red
  and green stay the push and the want).
- Combat: HP and MP on everyone, slow regeneration (1% a second). A skill has reach, effect, MP
  cost and cooldown: dps attack (reach 2, 10 damage, 5 MP, 1 s); the lurer's tag (reach 8, 1
  damage, 2 MP, 1 s); HP buff (+30 HP to a member under the threshold, 10 MP, 1 s); MP buff
  (+30 MP, 5 MP, 1 s), both reaching the buffer's ranged disk (8; the user 2026-11-02: the 5
  disk is only where a buffer moves, a teammate inside the ranged disk is recovered); a
  monster's bite (reach 1, 5 damage, 1.5 s). HP 100,
  MP 100. A dead monster is removed and counted; a dead player counts and returns to the centre
  after 5 s with full bars. Buffers never attack; thresholds default 90%.
- Monsters (decided by the user): idle at the spawn; aggro when hit **and on sight within a
  radius** (slider, default 6); the target is the player who **dealt the most damage** (a
  threat table per monster); chase with `pursuit`, stop at bite reach, bite on cooldown; leash
  back to the spawn when the target is further than 70.
- Roles: dps attack the nearest monster inside 10 (arrive at reach), never leave the disk
  (containment on the circle); buffers stay within 5 and buff the member with the lowest bar
  under the threshold and in reach; lurer v1 (D105's design): leave the centre when the dps
  disk holds at most 3 monsters, pursue the nearest untagged monster inside 50, tag it, keep
  tagging the next nearest, run back when 4 chase or none is left, wait 2 s, repeat.
- Speeds: lurer 4, dps 3, buffers 2, monsters 3.5; max force 8 (Pedestrians'). Constants first,
  sliders when tuning.
- Metrics (labels, pinned in tests): kills a minute, monsters delivered into the dps disk, the
  lurer's share of time inside 10, trip period, lost chasers, team deaths, the lowest HP a
  teammate reached.

Steps:
- F1. [done, #46] Demo "Training Field" (`steer_demos::TrainingField`,
  id `training-field`, last of the 2D demos): the map edge, the three disks with labels, the
  standing team (lurer on the centre, dps 3 out at thirds, buffers 2 out between them), monsters
  spawned by a seeded `fastrand` over the map outside the dps disk, the Monsters slider (the
  count held every step), the legend with the body colors. Tests: the six roles inside their
  disks and apart; spawns inside the map, outside the dps disk, spread across the lurer's
  radius; the slider's count. Mutation (spawns allowed inside the dps disk) fails the spawn test.
  Then (the user 2026-11-02, "looking good"; no PR until the environment is done): bodies 2 units
  across (`BODY_RADIUS` 1, a person at the map's scale); gaits at human speeds, 1 unit = 1 m
  (`Gait`: standing 0, walking 1.5, running 5; max force 8), every body wandering at its gait's
  speed inside its own disk (`contain` on a 40-point circle `role.radius() - BODY_RADIUS`, probe
  1 s ahead; a runner also brakes at 4 while the push is on, since its turning radius of about 3
  is wider than the buffers' disk; a walker turns within 0.3 and needs no brake), monsters idle
  the same way within 6 of their spawn. No collisions between bodies (the user). Then (the user):
  players always run; a monster draws a gait at random and keeps it 2-6 s. Measured and pinned:
  running, the whole body stays inside (at worst 0.44 inside the ring), the lurer averages 4.6,
  the dps 2.8-3.2, the buffers 2.2, nobody below 1.49; monsters spend about a third of the time
  in each gait, every one visits all three in two minutes, none faster than its gait or further
  than 4.4 from its spawn. (Measured before the change: walking, everyone holds exactly 1.5.)
  Mutations (the players' region without the body margin; a gait never redrawn) fail.
  Then (the user): the HP and MP buffers in their own body colors (dps blue, HP rose, MP teal);
  sight as "a fan of raycasts in front of an entity" (`fan`, `ray_enters_circle`,
  `TrainingField::sight`): 7 rays over 90 degrees ahead, 12 long, each cut at the near side of
  the first body it enters (`Ray { from, to, hit }`, `hit` an index over the team then the
  monsters), drawn in `color::SIGHT`. Tests: hand-computed entries (a radius-1 body 5 ahead is
  entered at 4; 0.6 aside at 4.2; aside, behind or beyond the range: none), the fan symmetric
  about forward, the lurer's middle ray cut at a monster 6 ahead, no ray ever hits its own body.
  Mutation (the far root of the intersection) fails the entry test.
  Then (the user: "the dps never go inside the buff disk, just around a band"): wander held in
  by containment circles along the edge (the probe leaves, the push turns the body along the
  wall), so the idle walk is now the game bots' random walk (`roam`): seek a random spot of
  the disk (uniform over its area), draw the next within 1.5 of it; containment stays as the
  guard. Measured and pinned: the dps spend 42-49% of the time inside the buffers' disk, the
  buffers all of it, the lurer 5%; speeds now average 4.3 (lurer), 2.7 (dps), 1.5 (buffers),
  never below 0.76 (a runner slows to turn back). Mutation (spots never redrawn) fails.
  Then (the user changed their mind: no sight fan): every entity carries two disks on its own
  position, `RANGED_REACH` 8 (big: a player's attack, a monster's alarm) and `MELEE_REACH` 2
  (small: a monster's attack), drawn in `color::RANGED` / `color::MELEE`; players are ranged;
  a monster with a player inside its big disk closes on the nearest one (`player_in_reach`,
  `hunt`: a run by `seek` while outside melee reach, a stop inside it; `Monster::target`, a
  `color::TARGET` line to it), else idles as before (back from a chase the spot leads home, the
  containment guard only inside the idle disk). Tests: from 7 away the hunt ends standing at
  the small disk's edge (gap within 0.1 of 2); the nearest player in reach is picked, none
  beyond 8; chases happen in the first minute and only from inside the big disk; in two minutes
  24 of 30 monsters never chased and those kept their gait shares and idle radius. Mutation (a
  stop at ranged reach) fails the hunt test. Then (the user): a monster's big disk is its
  alarm, `ALARM_REACH` 6, smaller than a player's 8; the buff rule above. The reach rings made
  a frame 3004 lines and 129 KB (Boids: 49 lines, 8.9 KB), 7.7 MB/s at 60 frames: circles are
  now a wire primitive (`CircleView`, `FrameView::circles`, `annotation::circle_view`), the
  browser draws each as 32 segments (`draw.ts`), the Ground lift turns them into rings, and the
  legend test checks circle colors too. The other demos still send their few circles as lines.
- F2. Combat, agreed with the user 2026-11-02 (the numbers in the table below), built in
  pieces: the bars, the monster's bite with the threat table and deaths, the dps AoE, the two
  buffs, the lurer's tag. Bars (built, #46): `MAX_HP` / `MAX_MP` 100, players regain
  `REGEN` 1 of each a second, monsters nothing; HP and MP bars over every body (`bar`: body
  width, 1.6 and 2.0 above the centre, the buffers' colors). Tests: regeneration 50 -> 60 in
  10 s, a monster stays at 50, bars counted and the half bar's length. Mutation (monsters
  regenerating) fails. Bars are twice the body's width in three strands (the user: too small).
  The bite (built, #46): `BITE` 3 every `BITE_COOLDOWN` 1.5 s on the target inside
  melee reach; `Monster::threat` (damage dealt per player) picks the living player with the
  most (`threat_target`), else the nearest in the alarm disk; a player at 0 HP dies
  (`Player::dead_until`, drawn in `color::BODY`, every monster forgets it), back at the centre
  with full bars after `PLAYER_RESPAWN` 5 s; a monster at 0 HP is removed and counted
  (`kills`), its replacement due after `MONSTER_RESPAWN` 3 s (`respawns`). Tests: a parked
  lurer loses 3, then 3 more each 1.5 s, less regeneration; a dead player is forgotten, gray,
  and back at the centre after 5 s, not 4.5; the threat pick (most damage, dead skipped, lowest
  index on a tie) and a monster hurt from 86 away running at the lurer; a killed monster's
  replacement after 3 s, not 2.5. Mutation (no bite cooldown) fails the bite test.
  The dps attack (built, #46): a dps targets the monster inside the dps disk nearest
  to it (`nearest_monster_in_disk`, `Player::target`), runs into its ranged reach (`hunt` now
  takes the reach) and standing fires every `AOE_COOLDOWN` 1.5 s for `AOE_MP` 6 (waits
  without the MP): `AOE_DAMAGE` 10 off every monster within `AOE_BLAST` 2 of the target, each
  hit added to that monster's threat for the shooter; drawn as a `color::ATTACK` line to the
  target and the blast circle. Tests: a monster 4 below the centre is in reach of all three
  dps, loses 30 a volley, holds threat 10 for each, chases dps 1, dies at the fourth volley
  and is replaced; two monsters 1.5 apart both take the blast, one 3.5 away does not; a dps
  short of MP holds fire until regeneration; a monster inside the disk but 10.3 from a dps is
  run at and hit within 2 s. Mutation (the blast hitting only the target) fails.
  The buffs (built, #46): `cast_buff`, once a second (`Player::skill_ready`, shared
  with the dps attack) when the buffer has the MP (`HP_BUFF_MP` 4, `MP_BUFF_MP` 2), on the
  living member within its ranged reach with the lowest bar under `BUFF_THRESHOLD` 0.9 and no
  such buff running, itself included; the buff (`hp_buff_until` / `mp_buff_until`) adds
  `HP_BUFF_RATE` 4 or `MP_BUFF_RATE` 3 a second to the regeneration for `BUFF_DURATION` 10 s;
  shown as a ring round the body (1.3 rose, 1.5 teal). Tests: a dps at 50 HP and one at 50 MP
  are buffed at once, read 75 HP and 70 MP at 5 s, full HP and 90 MP (the threshold, so no
  fresh cast) at 10 s, no second cast meanwhile; nothing cast at 95%, nor on a member 40 away;
  three in need are served lowest first, one cast a second, when in reach of the roaming
  buffer, the buffer itself first. Mutation (the HP buff adding nothing) fails.
  Line of sight (the user 2026-11-02: "it will keep chasing if the player is still in LOS",
  a third, biggest disk, 30-40): `LOS_REACH` 35; each step a monster forgets (threat and
  chase) every player beyond it, keeps its current target inside it even outside the alarm
  disk, and picks by threat, then the kept target, then the alarm disk; drawn in
  `color::SIGHT` on chasing monsters only. Tests: alarmed at 5, still chasing at 25, forgotten
  at 45 with its threat cleared and no sight circle; the chase test's bound is now the line of
  sight; the roaming measurements run on a quiet field, since dps now stop to fight monsters
  that follow players in. Mutation (no sight limit) fails.
  The lurer's tag (built, #46): `untagged_in_reach` (the nearest monster within the
  lurer's ranged reach with no threat of the lurer's on it), every `TAG_COOLDOWN` 1 s for
  `TAG_MP` 2, `TAG_DAMAGE` 1 off it and onto its threat for the lurer; the lurer keeps roaming
  (no stop to tag); its next tag drawn as the `color::ATTACK` line. Tests: two monsters 7 from a
  parked lurer are tagged a second apart, the nearer first, each once (the first is never
  scratched again over 3 s), both then chase and bite the lurer; no MP, no tag. The dps tests
  now count the lurer's tag on a monster 4 from the centre. Mutation (the tagged filter
  dropped) fails.
- F5 (built, #46): lurer v1, D105's design (`Lure`, `LureStage`, `lure_step`): at the
  centre it stands `CENTRE_WAIT` 2 s and leaves when the dps disk holds at most `LEAVE_LOAD`
  3 monsters; out, it runs to the nearest monster in its 50 disk it has not tagged
  (`next_to_tag`; the tag fires on its own within reach 8) and on to the next, turning back
  when `BACK_CHASERS` monsters target it (4 then; 3 since F6b), its HP is under `BACK_HP` 40% or nothing is left
  to tag; back, it runs home (`HOME` 1) and waits again. Metrics (`count_deliveries`, the
  second bottom label): trips, delivered (a monster chasing the lurer that enters the dps
  disk, once), lost (one that stops chasing outside the disk), the lurer's share of time
  inside the dps disk. Measured, two minutes of the default field: 4 trips, 10 delivered, 0
  lost, 12 kills, no deaths, 32% of the time inside the dps disk (pinned with margins: at
  least 3, 6, half, under 50%, at least 8 kills). Tests: the leave at 2.0-2.1 s on a quiet
  field (and straight back, nothing to tag), no leave with four monsters in the dps disk; back
  on four chasers (the step after they start chasing), at 30% HP, or with nothing left, then
  home. Mutation (no return on chasers) fails. Still roaming: dps and buffers; the lurer no
  longer does, so the roaming test covers those five.
- F6 (the user 2026-11-02, five features; the order recommended and accepted): rarities, the
  5-monster AoE cap and the lurer's two AoE skills, balance sliders and leaving at 100% HP, a
  tour planner for the lurer's walk, the AoE-aware planner. Already in: monsters switch to
  the dps in the disk (threat: a tag 1, a dps hit 10).
  - F6a (built, #46): `Rarity` (Normal, Champion, Giant, PartyNormal, PartyChampion,
    PartyGiant). HP multipliers are the game client's, read by caravan (sro_client.exe 37
    `FUN_00cbe7c0`, `views::hp_multiplier`, conventions.md): x1, x2, x20, and the party flag
    x10 more (x10, x20, x200); live, a party Uneg had 378,440 HP = 37,844 x 10. Damage
    multipliers are ours (caravan has none): x1, x1.5, x3, the party flag x2 on top (x2, x3,
    x6). Bodies: champion x1.25, giant x2. Spawn mix (`SPAWN_MIX`, percent): 80 / 12 / 3 /
    4 / 1 / 0 (party giants off: 20,000 HP is 17 minutes for this team). A label names any
    non-normal monster; bars fill by the monster's own max. The leash: a monster inside its
    idle disk with no target and no threat is whole again (`Monster::at_home`), as the game
    resets a leashed monster. Tests: the multipliers; 10,000 draws within a point of the mix
    and no party giant; a giant's body 2, label, half bar at 1,000 HP, bite 9; a hurt idle
    monster healed at once, one dragged 20 out healed once home, a chasing one not. Mutation
    (the party flag's x10 dropped) fails.
  - F6b (built, #46): every area attack hits at most `AOE_TARGETS` 5 monsters, the
    nearest to its centre (`in_blast`) or along its line (`along_pierce`). The lurer's kit
    (`lure_cast`): the normal attack (the tag), a blast (`LURE_BLAST` 3 round a monster within
    reach 8, 6 MP, 4 s) and a pierce (`PIERCE_LENGTH` 12 toward a monster, `PIERCE_WIDTH` 1.5
    either side, 8 MP, 6 s), each hit a tag (1 damage, the lurer on the threat table); it casts
    whichever catches the most untagged monsters when that is two or more, the pierce on a
    tie, else the normal attack; the label counts tags, blasts and pierces. N (`BACK_CHASERS`)
    is 3 by default (the user). Tests: six monsters in a row under the three dps and the
    lurer, five hit (69), the sixth untouched; one monster gets the normal attack, three
    spread within 3 the blast (all tagged, 6 MP), four in a line the pierce (all tagged, 8
    MP). Mutation (no cap) fails.
  - F6c (built, #46): `Settings` and sliders: N (`chasers`, 1-10, 3), the monsters'
    running speed (3-8, default 6: faster than the players' 5, as in the game; `speed_of`
    feeds `hunt` and `roam`), the bite's damage (1-10, 3) and reach (1-5, 2; the monsters'
    small disk follows it), the load cap (0-10, 3; 0 none). The leave rule (the user): the
    lurer leaves the centre as soon as its HP is full and the dps disk is within the cap; the
    2 s wait and `centre_since` are gone. Tests: at full HP the trip starts on the first step;
    at 85 HP it waits for the buff and regeneration (3 s, left at step 170-190); four parked
    monsters hold it, not with the cap off; the sliders set N and the speed, a bite of 10 at
    reach 4, a chaser's top speed 8; the params carry the values. Measured, two minutes: 4
    trips, 13 delivered, 1 lost, 10 kills, no deaths, the lurer inside the dps disk 56%
    (faster monsters catch it on the way home). Mutation (the leave ignoring HP) fails.
  - F6d (built, #46): the leave at `LEAVE_HP` 90% (the user: cut the rest). The tour
    planner (`plan_tour`): the N stops among the `TOUR_CANDIDATES` 8 nearest untagged monsters
    in the lurer's disk, in the order that minimises the run from the lurer through them and
    home, an exact depth-first search with pruning (at most 8! leaves), planned again every
    `PLAN_PERIOD` 1 s and whenever the tour empties, tagged stops dropped; stops are monster
    ids (`Monster::id`, stable through deaths), drawn as a `color::PATH` polyline to the
    centre. Session metrics (`Lure`): sessions done, mean session time, HP back home (mean,
    lowest), distance walked per delivered; a third bottom label. Tests: hand-computed tours
    (two stops: the right pair, 22.2 over 40; three: the left one last, 42.1 over 42.2; more
    asked than exist; none); a row of three to the right with two far off: the tour is the
    row nearest first, the lurer sets off along it, four tour lines, one session recorded with
    its time and HP; at 85 HP the leave comes after 1 s. Measured, two minutes: 5 trips, 9
    delivered, 3 lost, 11 kills, 1 death, 47% inside; 4 sessions of 16.5 s, back with 96 HP
    (lowest 93), 37 walked per delivered: the baseline F6e must beat. Mutation (the planner
    keeping the longest tour) fails.
  - F6e (built, #46): the cast planner (`plan_casts`, `Planner::Casts`, the default;
    buttons switch to `Stops`): the shortest run that tags at least N monsters by casts, an
    exact search over the 8 candidates where each stop is a monster to get within a skill's
    reach of: the normal attack (reach 8, that monster), a blast (reach 8, every untagged
    within 3 of it) or a pierce (reach 12, every untagged on the line from the cast point to
    it), the area skills only when ready at planning and only when they tag two or more, the
    rule `lure_cast` fires them by (a stop tagging one by pierce left the lurer standing at 12
    with nothing it could cast: found by a trace, the first design's stall); a run ends when N
    are tagged or nothing more can be, and a node with any way on is never an end, pruned or
    not (the second bug: a pruned branch counted as finished and a cheap one-tag stop won).
    `Stop { id, reach }` replaces the id tour. Tests: a cluster of three and a lone monster,
    N 3: one blast stop at the cluster, without the skills three normal stops on it; a radial
    row of four wanted whole: one pierce at the far end from 12 short (with three wanted a
    normal attack and a shorter pierce, 28 over 34, which the planner rightly prefers); the
    buttons. Measured, two minutes, the same seed: stops planner 9 delivered, 3 lost, 16.5 s
    a session, 37 walked per delivered, 47% inside; cast planner 12 delivered, 1 lost,
    19.6 s, 29 walked per delivered, 39% inside, back with 98 HP (lowest 94), 11 kills, 1
    death. Mutation (the blast never offered) fails. The legend folds behind a button and the
    status lines start at the map's left edge (the user: the legend covered the disks).
  - F6f (built, #46; the user: "the lurer is inside the dps disk since the luring
    disk doesn't have any monsters"): two causes. Respawns landed anywhere on the map, so
    the lurer's disk drained while nests outside it never fed it: a killed monster now
    respawns at its own nest after 3 s (`respawns: Vec<(time, nest)>`, `spawn_at`), as the
    game's nests are fixed; the slider's extra monsters still spawn at random. And with
    nothing to tag the lurer dithered (Centre, Out, Back every few steps, inflating the trip
    count): it now leaves only with something in its disk to tag (`tour_candidates` not
    empty). Tests: a trip at once with a monster 40 out, none within 3 s on a quiet field,
    the 85 HP wait, the parked four and the cap; a killed monster's replacement at the same
    nest with a new id. Measured, two minutes: 6 trips, 20 delivered, 1 lost, 15 kills, 4
    deaths, all dps (20 brought in against 15 killed: the overload is for the sliders, N and
    the load cap), 56% inside; 6 sessions of 14.8 s, back with 99 HP, 16 walked per
    delivered. Mutation (respawns anywhere) fails.
  - F6g (built, #46; the user's four changes): the map is a disk (`MAP_RADIUS` 60,
    nests an idle radius inside; a rectangle left monsters stuck in its corners); the lurer
    sees only monsters within `LURER_SIGHT` 30 (the game shows entities within about 30), and
    with none in sight scouts random spots of the band between 30 and the luring disk's edge
    less 3 (`scout_spot`, `Lure::scout`, drawn as the tour line); having tagged something and
    seeing nothing more it turns home with what follows (`left_casts`); the luring disk fences
    the lurer's walk, not its targets: the cast planner takes a cast point only inside it
    (`fence`), so a monster at 55 is a normal attack from 47 and one at 62 a pierce from the
    edge; a chasing monster inside the dps disk shows no line of sight. The leave rule no
    longer asks for work in the disk (scouting finds it). Tests: spawns inside 54; a quiet
    field sends the lurer scouting past 30 and never past 50; back with what follows; the
    fence cases; the sight list at 29 and 31; the hidden LOS. Measured, two minutes: 7 trips,
    24 delivered, 2 lost, 21 kills, no deaths, 70% inside (short trips, the load cap holding
    it); 7 sessions of 11.9 s, back with 98 HP (lowest 97), 11 walked per delivered.
    Mutation (no fence) fails.
  - F6h (built, #46; the user): two lurers (A and B, `LURERS`, the first two players)
    and four dps; `lures: Vec<Lure>`, metrics and status lines per lurer. Not kiting each
    other's monsters from what a lurer can see: a monster any lurer has tagged is off limits
    (`tagged_by_a_lurer`); an untagged one belongs to the nearest living lurer, the lower
    index on a tie (`owner`), applied to the next tag, the planner's candidates and the cast
    centres; scouting spots go on the half of the band away from a lurer that is out. Watching
    before starting, without deadlock: A never waits; B waits while an earlier lurer is alive
    within `WATCH_DISTANCE` 15 of the centre and out or ready to leave, at most `WATCH_TIMEOUT`
    5 s (`Lure::ready_since`). The strong cap (`Settings::strong_cap`, slider 0-5, default 1):
    a lurer stays while the dps disk holds that many strong monsters (`Rarity::is_strong`, HP
    x10 or more: giants and every party monster). Tests: the owner rule (nearer, tie to A, B
    dead); a pair of monsters each side, A tags only the left and B only the right, nobody
    both; B leaves when A is 15 out (steps 120-300) and on the timeout with A parked; a party
    monster in the disk holds both lurers, not with the cap off; eight roles; the dps tests
    with four dps (a monster 4 under the centre loses 41 a volley, dies at the third). Measured,
    two minutes: A 3 trips, 10 delivered, 0 lost, 16.1 s a session, 18 walked per delivered;
    B 4 trips, 16 delivered, 1 lost, 8.4 s, 6 walked per delivered; 26 kills, no deaths; both
    inside the dps disk most of the time (73% and 93%: the load cap holds them while the four
    dps clear). Mutation (the owner rule dropped) fails.
  - F6i (built, #46; the user: the lurers are independent, know nothing of each
    other's plan, and see each other only as nearby entities): the first build read the other
    lurer's stage and its position at any distance. Now each lurer gets its own view
    (`Snap`, `observe`, `Observed`): the other's position and movement only within
    `LURER_SIGHT` 30, its HP from the party window at any distance (an assumption about the
    game's party window), never its stage; a lurer remembers where it last saw the other for
    `SEEN_MEMORY` 10 s (`Lure::last_seen`, `Lure::new`). Ownership counts only the lurers in
    sight (two lurers closing on one monster see each other before either is in reach); the
    watch waits for an earlier lurer in sight within 15 of the centre that is healthy or
    moving; scouting goes away from the last sighting. Tests: A parked 40 out is unseen by B
    at the centre, so a monster at 20 is B's and B tags it; A parked 25 out is seen and nearer,
    so A tags it and B does not; `observe` itself. Mutation (the sight limit dropped) fails.
  - Default monsters 15 (the user 2026-11-03). The load cap measurement at 15, two minutes:
    cap 3: A 2 trips, 5 delivered, B 3 trips, 9 delivered, 16 kills, 5 deaths; cap 6: 13
    kills, 6 deaths; the cap stays 3. The deaths are all dps, to a giant and a party champion
    that reached the dps disk: 2,000 HP against four dps spreading their fire over different
    targets (27 damage a second at best, 75 s to kill), biting 9 every 1.5 s at dps that roam
    out to 10 while the HP buffer reaches 8 from within 5. Open (to decide with the user):
    the dps focusing one target; the lurers never tagging a strong monster; the buffers'
    reach against the dps disk. Decided (the user 2026-11-03): no focus fire (AoE dps); the
    lurer judges from the dps disk and the party window instead of a blanket "never"; the
    buffs are extended rather than the buffers' reach.
  - F6j (built, #46): the lurer's risk check, from what it can see (the disk's monsters
    within 30 of the centre) and the party window (`party_hp`, the mean HP percent with the
    dead at 0, kept per step over `TREND_WINDOW` 5 s; `party_drop`): a strong monster is a
    candidate, a normal-attack target or part of a blast or pierce only while the disk holds
    fewer strong than the cap, the party's mean HP is at least `RISKY_HP` 80 and it fell at
    most `RISKY_DROP` 10 in 5 s (`strong_ok`, through `tour_candidates`, `untagged_in_reach`,
    `lure_cast` and `plan_casts`'s `avoid_strong`); under `HALT_HP` 60 or a fall over
    `HALT_DROP` 20, no session starts (`halt`). The team label shows the party HP and its
    change. Tests: calm, A tags a normal and a giant 20 out; with a party monster in the disk
    the giant is left and the normal tagged; a 15-point fall blocks the giant for the window
    and A takes it on a later trip; at 50% no trip for a second, trips once over 60%. Mutation
    (`strong_ok` always true) fails.
  - F6k (built, #46): the HP buffer's further skills, tried in this order before the
    HP buff, still one cast a second (`cast_heal`, `cast_group_heal`, `cast_guard`;
    `members_in_reach`, `spend`): a heal (+40 at once, 15 MP, 3 s) on the member in reach
    lowest under 50%; a group heal (+20 to every member in reach, 25 MP, 10 s) when the
    party's mean HP is under 70%; a guard (bites times 0.8 for 20 s, 10 MP, 5 s; the user:
    the first draft's one-a-second blanket was too strong) on the unguarded member in reach
    that a monster targets, the lowest HP first; a guarded member wears a `color::GUARD`
    ring; the team label counts the casts. Tests: a dps at 40% is healed to 80 at once, the
    next one gets the HP buff while the heal cools and the heal after it; the party at 65%
    gets a group heal of 20, lurer A 40 away misses it; a monster after dps 2 brings the guard
    onto dps 2 the step after it turns; a guarded lurer loses 2.4 to a bite, not 3. Measured,
    two minutes at 15 monsters: the same trips (A 2/5/2, B 3/9/2), 16 kills, no deaths (5
    before), 9 heals, 0 group heals, 10 guards. Mutation (guard factor 1) fails.
  Combat numbers: bite reach 2, 3 damage, 1.5 s (one monster kills an unbuffed player in
  50 s, four in 12.5 s); dps AoE reach 8, blast radius 2 round the target, 10 damage to
  every monster in it, 6 MP, 1.5 s (one dps kills a monster in 15 s, three clear a cluster of
  five in ~5 s; 4 MP/s spent against 1 regained); tag reach 8, 1 damage, 2 MP, 1 s; HP buff
  +4 HP/s for 10 s, 4 MP; MP buff +3 MP/s for 10 s, 2 MP; a buffer casts once a second on the
  member (itself included) with the lowest bar under 90% and no buff running, within its
  ranged disk 8. Threat = damage dealt per player (a tag 1, a dps hit 10: the first dps hit
  takes a monster off the lurer). A dead player returns to the centre after 5 s with full
  bars and is forgotten; a dead monster vanishes, a new one spawns elsewhere after 3 s; a
  monster back home is at full HP.
- F3. The dps role. Verify: never outside 10 (plus the body), kills happen.
- F4. The buffers. Verify: bars stay at or above the threshold while the buffer has MP.
- F5. Lurer v1 and the metrics; measured numbers pinned.
- F6. Further lurer strategies side by side (PhBot's random walk, caravan's state machine),
  switched by buttons under the same metrics.

## Each demo page

three.js view, play/pause/step/reset, parameter sliders, toggles for each annotation. Demos are listed
in a sidebar, mirroring OpenSteerDemo's plug-in selection.

- [done] Play/pause/step/reset: calls `sim.play|pause|step|reset`; frames carry `running`. Step = pause + one 1/60 s step, as OpenSteerDemo. A paused sim sends no frames.
- [done] Demo selection: sidebar -> `demo.select` (unknown id refused with an error), demo enum dispatch (2D `Demo`, 3D `Demo3`), frame carries the current `demo`.
- [done] Demo input: `demo.click {x, y}` (2D scene raycasts the click onto the ground; 3D demos take no clicks) and `demo.action {id}` for the buttons each frame lists in `actions`.
- [done] Legend: each frame lists `legend` (color, meaning); one meaning per color across 2D and 3D demos (`steer_demos::color`); a test per demo crate fails if a demo draws a color its legend does not explain. Labels are neutral gray.
- [done] Parameter sliders: each frame lists `params` (id, label, min, max, step, value); `demo.param {id, value}` sets one; the demo clamps to the range. Shown next to the demo's buttons.
- [done] 3D view: drag to orbit, scroll to zoom; the camera is placed afresh per demo and then follows `focus`.
- Annotation toggles: when a demo needs them.

## Decisions and open questions

Decided with the user (2026-10-21):
- World (step C): Jangan (0x61a8) and its 8 neighbors; first demo 3D Seek on the ground; terrain
  to the browser as a Rust view model (untextured mesh, sent once per demo), not caravan's
  `~/.caravan/map` assets; the navmesh copy in `crates/silkroad-navmesh`, frozen at caravan
  `0ad7987` (re-copied only for a needed fix, recorded in `SOURCE.md`); our additions in new
  modules there.
- Walls (step D): OpenSteer `RectangleObstacle`s with the ported avoidance; a walker reacts only
  to walls of its own floor; the wall query in `silkroad_navmesh::walls`; the first demo is the
  Plaza. When chained wall pieces broke the rectangle hints, the user chose merging straight runs
  (over sliding along walls or switching to containment).

Decided with the user (2026-11-02 to 03, Phase 8, in order):
- A 2D demo in `steer-demos`, not on the Silkroad terrain; bodies 2 units across; human gaits
  (1 unit = 1 m); no collisions between bodies; players always run, monsters draw gaits; the HP
  and MP buffers in their own body colors; no sight fan: two disks per entity, ranged 8 and
  melee 2, a monster's alarm 6 smaller than a player's 8; the buff disk 5 is only where a buffer
  moves, buffs reach the ranged disk; a chasing monster keeps its target within a third disk of
  30-40 (35); the combat numbers as recommended (HP/MP 100, bite 3 per 1.5 s, dps AoE 10 for 6 MP,
  buffs over time); threat by damage dealt, so the first dps hit takes a monster off the lurer;
  monsters attack on sight; the bars' dark track.
- The five features and their order (rarities, the five-monster AoE cap with two lurer AoE
  skills, balance sliders with the leave at full HP, a tour planner, the AoE-aware planner);
  rarities only the six named, HP multipliers the game client's, damage multipliers and the mix
  mine; N = 3 by default; the leave at 90% HP; the map a disk (corners held monsters); the game
  shows entities within about 30, so the lurer sees 30 and scouts the band beyond; the luring
  disk fences the lurer's walk, not its targets; no LOS circle for a monster fighting in the dps
  disk; 15 monsters by default.
- Two lurers and four dps, the party's limit of 8; the lurers are independent, know nothing of
  each other's plan, and see each other only as nearby entities (the party window gives HP);
  ownership by the nearer visible lurer, B watches A before starting with a timeout, a strong
  cap. No focus fire for the dps (AoE parties do not need it); the lurer judges a strong monster
  from the dps disk and the party window rather than a blanket "never"; more buff skills rather
  than a longer buffer reach; the guard toned down to 20% for 20 s, 10 MP, 5 s (a one-a-second
  blanket was "quite outstanding"). The load cap stays 3 (6 measured worse at 15 monsters).
- After the browser observation (2026-11-03): a monster a dps's damage takes over within the
  blast's 2 outside the disk counts delivered, not lost (every lost in the seed-1 run was such a
  packmate); the strong cap default goes from 1 to 2 before the obstacles; the obstacles'
  design as settled in Phase 8, "Next step: obstacles", with no line-of-sight occlusion.
- Risk by the lowest bar, not the mean (the user, 2026-11-03: "not sure the party HP average
  is a good metric"; measured: the mean read safe through all 19 s a member spent under 50%):
  a strong pull only while every member is at least 70% and none fell more than 15 in 5 s; no
  session with a member under 50% or fallen more than 30. Next, the load as a backlog in
  seconds instead of head counts (what the dps still have to kill, visible as monster bars).

Open (ask before acting):
- The party window showing members' HP bars is my assumption about the game; if it does not,
  the lurers' risk check and the watch rule lose that reading.
- Lurer A delivers less than B in the one measured seed (the tie rule hands A the near
  monsters); alternate who leaves first, or leave it: measure more seeds first.
- The lurers' MP: sessions cost blasts and pierces and the MP buffer reaches a lurer only at
  the centre; no wait or return on MP yet.
- Arrive: keep the faithful "orbits unless braking is tuned" result, or add a non-OpenSteer stop rule?
- Obstacles: keep the faithful container widening (body crosses the arena wall by ~0.5), or shrink containers?
- Pedestrians (2D): add the plug-in's tilted rectangle as a 2D wall obstacle? (`Rectangle` now
  exists in 3D, oracle-checked; 2D has none.)

## Findings (verified, with numbers)

Integrator and fidelity:
- **Integrator per step**: low-speed bend -> cap at max_force -> /mass -> blend 15% (clip(9*dt, 0.15, 0.4)) into smoothed acceleration -> velocity += acc*dt, cap at max_speed -> position += velocity*dt -> forward = velocity direction (then `side`/`up` rebuilt).
- **One Turning:** speed dips from 1.5 to 0.963 at 2.80 s (0.832 at 4.25 s with the low-speed rule off), reaches max speed 5 at 20.7 s. The smoothed acceleration matches the capped push except during the first ~0.5 s (smoothing, 15% per step at 60 Hz) and ~2.5-4.5 s (low-speed rule).
- **The low-speed rule's `pow` must be computed in double.** C++ `pow(range, 20)` with a float and an int promotes to the double `pow`; the port used `f32::powi`, 1 ulp apart at times. Earlier oracles hid it under 1e-4; flocking amplifies it (boid 0 off by 2e-3 at step 200). Traced to the first bit difference (step 12, boid 6, low-speed regime); with `f64::powf` and the oracle built with `-ffp-contract=off` (clang on arm64 fuses multiply-adds by default) the traces are bit-identical over 7200 boid-steps. Oracles of chaotic scenes need `-ffp-contract=off`.
- **`side` is state in OpenSteer, not a function of `forward`.** `regenerateOrthonormalBasisUF` sets `side = normalize(cross(forward, up))` from the previous `up`, then `up = cross(side, forward)`; on the ground `up` stays (0, u, 0) with u drifting from 1 (e.g. 1.000002). The 2D port's exact `perp` was a few ulps off; soccer drifted from step 85 (first bit difference: player 4's avoidance push = side x steer) to 1.6e-4 by step 1080. With `Vehicle::up` + `regenerate_basis` the soccer trace is bit-identical over 3600 steps; all earlier oracles still pass. The 3D port stores the whole basis.
- **A wrong-handed basis hides on even steps.** With `side = up x forward` both `side` and `up` change sign every step, so checkpoints every 60 steps matched the C++ for a constant push; only the banking run (which uses `up`) caught it. 3D oracles check step 1 too.
- **Scenario runs reach few cells of a lookup table.** The 3D pursuit runs used 5 of the 9 time-factor cells; a mutation in an unused cell passed until a test set up all nine directly.
- **Sphere intersection in local space versus world offsets** differs only below 1e-4 (2D oracle, and 1800 steps in 3D): bit-level only. 2D uses world offsets, 3D the C++ local space.

2D behaviors:
- **Seek never stops, it orbits.** From rest at the origin to target (-3, 4) with max_force 1, max_speed 3: after 30 s it circles at 0.459 from the target, speed 0.583. The C++ prints the same digits. Cause, proven by switching it off: the low-speed rule (`adjustRawSteeringForce`, below 0.2 * max_speed bends the force toward forward) removes the braking component; with the rule off the vehicle stops on the target (distance 0.000). This is why Arrive exists.
- **OpenSteer's seek slows down by itself.** `xxxsteerForSeek` truncates the offset to max_speed, so inside max_speed units the desired speed equals the distance. Plain `steerForSeek` does not.
- **Flee has no "far enough"**: it runs away forever at max_speed.
- **Arrive alone does not stop either** (rule on, same setup): r=3 equals OpenSteer's seek to every digit; r=5/9/15 orbit at speed 0.583 (just under 0.2 * max_speed = 0.6), distance 0.5-0.64 at 30 s. With the rule off every radius stops (r=9 fastest, step 612). r >= max_speed^2 / max_force = 9 never asks for more than max_force.
- **Braking can stop Arrive, but only if it never passes the target.** Braking inside the radius: r=9 rate 0.75 and r=5 rate 1.5 come to rest and stay (checked to 120 s). r=5 rate 0.75 gets within 0.0003, creeps past (target cos +1 -> -1 at ~22.5 s), and the low-speed rule turns the pull back into a forward push: speed doubles about every second. Rate 3 holds speed at exactly max_force / rate = 0.333 (full forward push vs full braking) and wanders.
- **Wandering runs faster than the target speed.** Target 1.5, mean speed 10-60 s: weight 0 -> 1.500, 1 -> 1.521, 3 -> 1.669, 5 -> 1.901 (the C++ shows the same). Cause, proven by switching smoothing off (weight 3 drops to 1.517): the smoothed acceleration lags the turning side vector, so part of the sideways force points forward; the rest is Euler integration lengthening the velocity each turn step.
- **An equally fast pursuer never catches a wanderer.** Oracle: pursuer from (8, -6) behind `MpWanderer` (both max_speed 3) is still 3.04 behind after 10 s. MultiplePursuit's pursuers catch the wanderer only when it turns toward them.
- **Plain flee slows down near the menace.** `steerForEvasion` uses plain `steerForFlee`: desired speed = distance from the predicted position, so the evader (max 2) runs at 0.38-0.85 once the hunter is within ~1-2 units (oracle steps 120-600) instead of escaping.
- **A container lets the body cross its wall.** `SphereObstacle` widens every obstacle by the vehicle radius, also one seen from inside, so the arena (radius 15) is tested at 15.5: over 2 min the center reaches 15.02 (206 of 7200 steps beyond 15). Rocks keep a full body width (min clearance 0.5003). Shrinking the container by the radius instead keeps the center within 14.17 (not ported: not OpenSteer).
- **Mapping quirks (pathway).** Ties between segments go to the earlier one (strict `<`), so the corner (10, 6) maps to segment 1. On a cyclic path a negative distance is not wrapped: `fmod(-3, L) = -3`, then clamped to 0 (the start).
- **Only directed following without wander keeps pedestrians inside the path.** Ten pedestrians, 60 s after 10 s settling, worst distance outside: directed alone -0.092 (never out); directed + wander (the plug-in's default) 2.326; stay-on-path alone 1.939 (no direction, so they walk off the open ends: 1487 of 2682 outside steps within 1 of an end).
- **The eight's endpoints are 3.06 apart**, so with radius 2 a directed pedestrian at one end can flip direction twice in quick succession (oracle: -1 at step 2520, +1 again by 2700).
- **A neighbor grid must follow the boids during the pass.** A boid that wraps around jumps ~100 units; a grid built at the start of the pass missed it (step 284: boid 40 wrapped to 5.52 from boid 49). C++ updates each boid's proximity token after it moves; the port does the same (`Grid::move_point`).
- **In perpendicular encounters the faster vehicle gives way**, not the slower as the C++ comment says: the code tests `threat->speed() <= speed()`. The port follows the code.
- **Neighbor avoidance reduces but does not prevent overlaps.** 8 crossing pedestrians, 60 s: 847 steps with bodies overlapping without avoidance, 87 with it (57 before the `side` fix: chaotic). It reacts only to the earliest threat, and an overlap gets only a sideways push.
- **Capture the Flag is about even.** Random rounds over 10 min: 32 ended, 17 reached home, 15 tagged, mean round 14.7 s.
- **Pedestrians can start inside a rock.** The rocks sit on the path and `Pedestrian::reset` does not avoid them: 6 of 100 start inside (the C++ does the same). In 60 s nobody else enters one; this was proven by elimination (leak-through off: 600 vs 609 inside-steps; neighbor avoidance off: 623), then by counting starts (6) and new entries (0).
- **Soccer players avoid only earlier players.** Each `Player` copies `m_AllPlayers` by value when built, so team A's first player avoids nobody and team B's last avoids 15.

Phase 6 (1999 behaviors, 2D):
- **Separation is 0/0 for two vehicles on the same spot.** `steerForSeparation` divides each offset by its squared length. Leader Following with separation weight 0: followers pile onto the point behind and at step 1510 two are bit-identical, so separation (times 0) turned NaN. `follow_leader` (2D and 3D) skips separation at weight 0; the ported `separation` is unchanged.
- **Leader Following: separation spreads followers but they still overlap.** 8 followers, 60 s: weight 2 -> mean 3.88 from the point behind, 672 of 3600 steps with an overlap, 3.1% of follower-steps stepping aside; weight 0 -> 3.21, 3524 steps, 0.14%.
- **Wall following overshoots every 90-degree corner by about 1.4.** Offset 2, band +-0.5, look-ahead 1 s, 15-60 s: distance from the wall min 0.585, mean 2.306, max 3.385 (clockwise 0.814 / 2.348 / 3.405). Located by logging: the minimum is just after the inner corner (walkers run past the outline's turn at (1, 2) to x = -0.41, 0.59 from the wall edge x = -1), the maximum just after the outer corners (x = 9.37 vs the outline at 8). Path following only reacts once the predicted position leaves the band. Look-ahead 3 s cuts corners and rides closer (0.527 / 1.485 / 3.476); offset 4: 2.642 / 4.406 / 5.790.
- **Containment took four designs; each failure was traced to its first escape.** (1) The paper's push (the sideways part of the nearest edge's inward normal): past a convex corner two edges tie for nearest and their pushes alternate (+5 y / -5 y) and cancel. (2) The full inward normal (turn and brake): the wanderer slows to 0.58, under the low-speed rule's 0.6, which bends the inward push forward into the wall (rule off: 4389 -> 58 outside steps). (3) Sideways toward the nearest point: heading into a corner, that direction is nearly anti-parallel to forward, so its sideways sign flips with heading noise. (4) Side probes, turn toward the roomier one: the choice holds (turning toward room makes that side roomier); what is left is physical: at speed 3 and max force 5 the turning radius (~1.8) is too wide to get out of a corner.
- **2D containment needs braking at corners.** 5 wanderers in the L-shaped room, 60 s (escapes / wanderer-steps outside / furthest out): look-ahead 1 s, no braking 4 / 364 / 2.05 (all come back); brake 1: 3 / 86 / 0.64; brake 4: 0 / 0 / 0. Look-ahead 0.5 s: no braking 32 / 2029 / 0.94 (one still out at the end), brake 4: 1 / 31 / 0.13. Look-ahead 2 s, brake 4: 1 / 93 / 0.70. Only the default (1 s, 4) kept everyone in.

3D:
- **A banking vehicle circling to its side sinks.** Push `0.8 side + 0.1 forward`, banking on (C++ and port): y = -3.30 after 10 s. Rolling into the turn tilts `side` down (side.y = -0.025 at 10 s), so the sideways push gains a downward part; Boids counter this with their sphere constraint.
- **3D seek orbits like 2D seek, in a tilted plane.** From rest, max force 1, max speed 3, after 30 s: target (-3, 0, 4) on the ground gives distance 0.459, speed 0.583, y = 0 throughout (the 2D digits exactly); target (6, 2, -4) gives 0.460 and 0.583 on a circle tilted 18.4 degrees (plane normal (-0.316, 0.949, 0); y between 1.85 and 2.14).
- **3D Arrive orbits like 2D Arrive.** Toward (6, 2, -4), 30 s: Arrive alone (radius 9) orbits at 0.554 and speed 0.582, just under the low-speed rule's 0.6; braking (radius 9, rate 0.75) stops it (0.018, 0.004); radius 5 rate 1.5 too (0.002, 0.0007); radius 5 rate 0.75 still orbits (0.547, 0.566).
- **The 3D wanderer overshoots its target speed more than the 2D one, and drifts in height.** Target speed 1.5, mean speed 10-60 s (2D in brackets): weight 0 -> 1.500 (1.500), 3 -> 1.827 (1.669), 5 -> 2.191 (1.901). Over 60 s at weight 3 its height ranges -5.02..7.78, and its `up` has rolled to y = 0.446 without banking (climbing turns roll the basis). Cause of the larger overshoot not yet proven (3D has two lateral parts).
- **3D pursuit: 20 catches a minute; the evader is caught at 3.87 s.** 30 pursuers on a 20-30 shell around the 3D wanderer catch it 20 times in 60 s. Pursuit vs evasion (evader max speed 2, hunter 3, from (-5, -2, -5)): the hunter touches the evader at step 232, since plain flee slows near the menace (the 2D finding).
- **3D obstacle avoidance repeats the 2D container finding.** Wanderer in a container sphere 15 with three rocks, 2 min: the center reaches 15.022 from the container's center (widened by the vehicle radius, as in 2D); rock clearance stays at least 0.860 (body radius 0.5); avoiding 1142 of 7200 steps.
- **On a 3D path even directed following leaves the tube.** Roller-coaster eight, radius 2, 10 walkers, worst distance outside over 60 s after 10 s: directed 0.602 (2D: never out), directed + wander 1.705 (2D 2.326), stay on path 0.915 (2D 1.939), stay on path + wander 3.349. Likely the vertical curves (not proven).
- **3D Boids fly in all three dimensions.** 100 boids, 10 s: 93 wrap-arounds at the radius-50 sphere, heights -36.4..39.3, mean speed 8.59 of 9. The 30-boid oracle run matches the C++ over 600 steps (24 wraps).
- **3D crossing: avoidance cuts overlaps by 78%.** 12 walkers through the center of a radius-10 sphere, 60 s: 570 steps with bodies overlapping without avoidance, 125 with it (2D, 8 walkers on a circle: 847 and 87).
- **3D Leader Following: far fewer overlaps than 2D.** 8 followers, 60 s, separation weight 2: mean 4.23 from the point behind, bodies overlapping on 57 of 3600 steps (2D: 672), 3.3% stepping aside; weight 0: 3.96, 3523 steps, 0.7%. Likely because followers also spread vertically (not proven).
- **3D wall following never touches the wall.** Floating 12 x 6 x 8 block, three walkers, 15-60 s: gap 2 -> 1.68..3.37 (mean 2.37); 3 s look-ahead -> 1.16..2.57; gap 4 -> 3.82..4.87; gap 1 -> 0.62..2.52. Seeking the nearest point of the rounded offset surface turns corners smoothly, unlike the 2D outline version (overshoot ~1.4 per corner).
- **3D containment in a box room needs no braking.** 5 wanderers, 60 s: look-ahead 1 s -> 0 escapes with or without braking; 0.5 s with brake 4 -> 0; 0.5 s without braking -> 14 escapes, 812 wanderer-steps outside, at most 0.39 out, all back by the end. The room is convex (the 2D L-shaped room's inner corner is what needed braking) and the up/down probes add turning room.
- **A wall met dead center gives no avoidance push.** `PlaneObstacle`'s hint is the normal back out plus the crossing's direction from the rectangle's center (toward the nearer edge); at the center that second part is zero, the hint is straight back, its part sideways to forward is zero, and so is the push (unit test; the oracle prints the same hint (0, 0, -1)). Off center by 1 the hint is (1, 0, -1) and the push full force toward that edge.
- **Rectangle avoidance slips past at the widened edge.** Seeking back and forth across an upright 6x2 wall and a tilted 7x7 one (avoidance 1 s ahead, max force 5, speed 3), the vehicle never crosses a wall inside its outline widened by the radius (0.5): it goes under (y = -0.53), over (y = 2.50) or around the side (|x| = 3.51..3.57), so its body just touches the edge. A walker on the ground cannot go over or under (matters for D3).

Silkroad world:
- **The terrain under Jangan's town is not the ground.** On a 320-unit grid the terrain inside Jangan has pits of -33 to -65 under the buildings (the walkable floor is the object above; the plaza is 21.5 higher than its terrain, as in caravan). The 3x3 around Jangan spans -79.8..114.8 game units, steepest 3.8 per unit (cliffs). Terrain-only demos stay outside the town.
- **A target straight behind a resting vehicle is never reached.** At rest the low-speed rule limits the force to a zero-width cone around forward; a force exactly backward has no sideways part, so it becomes a pure forward push, and the vehicle drives away at 0.58 (the orbit speed of the Seek finding) for good. Found by starting "3D Seek on Silkroad" with the first target straight behind; the start was moved.
- **Seeking a target above or below weakens a level push.** Seeking the target's real height and flattening the push lets the vertical offset use up part of the capped desired speed, so the track differs from the flat run; seeking the target at the vehicle's own height keeps the track bit-identical to the flat one.
- **Standing on the nearest surface fails at every floor just above the terrain.** The game lists the terrain as walkable under the gate (floor 0.97 above it) and under the plaza (floor -32.6, terrain -33.2 to -54): a walker on the terrain keeps the terrain as the nearest surface and follows it down into the moat under the gate (first failure traced at x = 446, game units), then into the pits under the plaza. A coarse 10-unit line walk hid it (the terrain dropped 7 at once, so the floor became nearest). The step-up rule (highest surface at most 5 game units above the feet, else the nearest) keeps the gate, the ramp and the plaza (-32.6 on every leg).
- **The plaza floor has holes.** Along z = 854 the plaza floor is missing from x 880 to 1070 game units (a building or fountain, 200 units, past caravan's 60-unit bridging); without walls a walker there drops 21 units onto the terrain. Walls fence it (x 878.9 and 1072.3 at -32.6, step D2); since D3 the Plaza's second leg is aimed straight across it and the walker goes around.
- **`min_by` distance to `f32::MAX` cannot pick the highest.** `f32::MAX - h` rounds to `f32::MAX` for every h, so all surfaces tie and the first (the terrain) wins; the highest is taken with `max`.
- **Most blocked terrain edges lie on region borders, and are not walls.** Jangan's 23 blocked terrain edges are all one-way (0x02); 15 lie on its border (z = 0 or 1920) and are skipped as sro-polyanya does, the other 8 fence the region's two closed tiles. The town's walls are object edges: 664 blocked outline edges, 21 bridge rims, 77 blocked inline edges.
- **Rectangle avoidance steers into the joints of chained walls.** The navmesh splits a straight wall into short edges; OpenSteer's rectangle hint points toward the crossing's nearer end of each rectangle, and where two pieces meet both point at the joint. Traced on the plaza (walker aimed across the hole): two rectangles met at z = 84.28, both hit at the same distance within 1e-5 with opposite hints, the nearest switched by a hair and the push flipped between z = -8 and +8 until the walker crossed and dropped 2.2 into the hole. Merging straight runs fixed it (the hole's west side became one wall, 81.7..89.6); concave corners could still trap a walker (not seen yet).
- **The 3D Seek vehicle is too weak for walls.** With max force 1 at speed 3 (turning radius about 9) it pushed at full force for 2.4 units and turned 13 degrees before hitting a slab's wall; with Pedestrian.cpp's force 8, speed 2 and 6 s look-ahead it never crossed.
- **Avoidance only looks along the heading, so bodies scrape walls.** Passing a wall or a corner, the forward line no longer meets it and the seek pulls the walker in: on the plaza the body (radius 0.5) is over a wall on 299 of 9600 steps, and once the center comes within 0.0019 of a corner of the hole (it moves past the tip). Never through a wall into the hole.

Phase 8, the Training Field (seed 1, two minutes at 60 steps a second; A/B = lurer A/B;
trips/delivered/lost):
- **The two-minute run moved with every design step** (all pinned in the test's comment):
  lurer v1, normal monsters only, 30: 4/10/0, 12 kills, no deaths, 32% of the time inside the
  dps disk; with the rarity mix: 4/12/1, 12 kills, 1 death, 50%; monsters at 6 and the leave at
  full HP: 4/13/1, 10 kills, no deaths, 56%; the tour planner and the leave at 90%: 5/9/3, 11
  kills, 1 death, 47%, 4 sessions of 16.5 s, 37 walked per delivered; the cast planner: 5/12/1,
  11 kills, 1 death, 39%, 19.6 s, 29 walked; nests: 6/20/1, 15 kills, 4 deaths (all dps: 20
  brought in against 15 killed), 14.8 s, 16 walked; the disk map with sight 30 and scouting:
  7/24/2, 21 kills, no deaths, 70% (the load cap holding the lurer), 11.9 s, 11 walked; two
  lurers and four dps at 30 monsters: A 3/10/0 (16.1 s, 18 walked), B 4/16/1 (8.4 s, 6 walked),
  26 kills, no deaths, inside 73% / 93%; at 15 monsters: A 2/5/2, B 3/9/2, 16 kills, 5 deaths
  (load cap 6: 13 kills, 6 deaths); with the risk check and the heal, group heal and guard: the
  same trips, 16 kills, no deaths, 9 heals, 0 group heals, 10 guards; the splash rule (a
  packmate a dps's damage takes over within the blast's 2 outside the disk counts delivered):
  A 2/7/0, B 3/11/0, 23 / 11 walked per delivered, the rest unchanged (every former lost was
  such a packmate); strong cap 2: the same two minutes (the giant and the party champion
  arrive together at 50 s and hold both lurers until 120.5 s either way), at 180 s A 5/15/0,
  B 5/17/0, 24 kills, no deaths, at the centre 93 s / 113 s of 180 (cap 1: A 3 and B 4 trips,
  23 kills, 118 s / 123 s); 8 rocks: A 4/10/0, B 4/14/0, 17.0 s / 11.3 s, 27 / 11 walked per
  delivered, 61% / 83% inside, 22 kills, no deaths; every path moved, so a giant was in the
  disk at 40 s and two party champions by 80 s (each lurer planned its strong one while the
  disk held one), 6,000 HP that held both lurers from 76 s past 180 s (at the centre 112 s /
  135 s); bodies graze rocks (closest clearance 0.00 over 180 s, three grazes of the rock of
  radius 1.6), never inside; circles and rectangles (a coin per rock, the layout redrawn): none
  of the 8 lies on the first two minutes' paths, so the two minutes are the no-rock numbers; at
  180 s A 5/15/0, B 5/18/0, 28 kills, no deaths, at the centre 93 s / 94 s, closest clearance
  0.00.
- **Wander held in by containment circles along the edge**: the probe leaves, the push turns
  the body along the wall, and wander rarely turns it back, so the dps never crossed the buff
  disk. Seeking random spots of the disk (the game bots' random walk) put them inside it 42-49%
  of the time.
- **A frame of 3,004 lines (129 KB, 7.7 MB/s at 60 frames) came from 75 circles of 40 segments**;
  a circle primitive on the wire made it 6.4 KB.
- **The first cast planner stalled**: it accepted a pierce stop tagging one monster, which the
  cast rule never fires (two or more), and the normal attack needs 8 not 12, so the lurer stood
  10.8 from its stop for 25 s (found by a second-by-second trace). Aligning the planner's rule
  with the cast's fixed it. Its second bug: a branch whose continuations were all pruned counted
  as a finished run, so a cheap one-tag stop won; a node with any way on is never an end.
- **Respawns anywhere drained the lurer's disk** while nests outside never fed it; with nothing
  to tag the lurer dithered (Centre, Out, Back every few steps, inflating trips). Nests and
  "leave only for work" (later: scouting) fixed both.
- **At 15 monsters five dps died with everyone else at full HP**: a giant and a party champion
  reached the disk (2,000 HP against four dps spreading fire, 27 damage a second at best; a bite
  of 9 every 1.5 s against a dps roaming to 10 while the HP buffer reaches 8 from within 5). The
  heal, group heal and guard brought it to no deaths at the same seed.
- **OpenSteer's integrator takes a couple of seconds to reach a monster's top speed** (smoothed
  acceleration), and the low-speed rule bends the first steps toward the starting heading, so
  "heads toward" tests check after a second, not after two steps.
- **A mutation can pass by moving its own expectation**: the guard test compared against
  `GUARD_FACTOR`, the constant mutated; expectations must be literals.
- **Observed in the browser 2026-11-03** (the seeded default run, the same as the pinned test's
  and at real time; a 180 s timeline test confirmed every number). A leaves at once and runs
  36-40 m west for 20 s trips (5 delivered in two trips); B leaves 4.3-5 s after A and runs
  12-15 m east for 10-12 s trips (9 delivered in three). A giant and a party champion (2,000 HP
  each) reach the disk at 50 s and die in one blast at 120.5 s; a party normal (1,000 HP) sits
  there from 138 s on. With the strong cap at 1 (zero strong allowed) both lurers stand at the
  centre meanwhile: A 118 s of 180, B 123 s. Party HP never fell under 90%, no deaths. All 8
  "lost" events came within 0.1 s of a delivery: the dps blast splashes a pack's trailing
  monsters just outside the disk, their threat switches, and they count as lost; no monster was
  lost on the way. The status lines hung at y 63/67/71 while a landscape canvas shows 66: lurer
  B's line was never visible (fixed: 51/55/59).
- **A point probe overshoots a thin block.** The first rectangle avoidance (the mirror of
  `contain`, pushing while the ahead probe is inside the block) let a monster walk 2.3 into a
  slab 3 wide: at speed 6 the probe reaches 13 ahead, so near the slab it lands beyond the far
  face and reads clear. The trigger is now the line from the body to the probe crossing an
  edge (`Wall::crosses`); the side still comes from the side probes' room.
- **The party mean hid every danger.** Seed 1, 180 s with rocks: the mean never fell under
  91% and never dropped more than 8 in 5 s, while the lowest member reached 11% and one member
  fell 74 in 5 s; a member was under 50% for 19.2 s and the mean read safe (80% or more) the
  whole time. With the lowest-bar rules (70 / 15, 50 / 30): A 3/7/0, B 4/15/0, 17 kills, no
  deaths in two minutes; at 180 s A 3/11/0, B 4/15/0, 21 kills, at the centre 114 s / 139 s
  (halts at 50, 80 and 120 s on a dps under 50%, which the mean never saw).

## Gaps and known issues

- `steerd` does not serve the built web app (`apps/web/dist`); only Vite does (`scripts/dev.sh`).
- One shared sim for all browser clients: one current demo and one running state.
- steerd's port (7400) and Vite's (5173) are fixed, so a second copy cannot run beside the user's `dev.sh`; the user checks views in their own browser.
- Not confirmed in a browser by the user: the 2D demos of steps 9-23 (incl. Arrive/Brake, the One Turning push line). The 3D view and all 3D demos were reported fine.
- Annotation lines are 1 px (WebGL `LineSegments`); three's `Line2` fat lines if too faint. No annotation toggles yet.
- Sidebar is a fixed 180 px column (not phone-friendly); labels overlap when a vehicle is near its target; the page has no favicon (404 in the console).
- The C++ OpenSteerDemo app (CMake build) was never built here; its cppunit tests are not wired into CMake.
- Pedestrians lacks the plug-in's third obstacle, a 7x7 `RectangleObstacle` tilted 45 degrees (seen from both sides): on the ground it is a wall at x = 20, z -3..4, but its steer hint has a vertical part that shrinks the in-plane push after `setYtoZero`, so a faithful 2D port needs a dedicated wall obstacle.
- Pedestrians and Boids take neighbors in index order; OpenSteer's LQ database returns them in bin order, which matters only for which of several equal threats is answered first.
- 3D Boids looks at every other boid (no 3D proximity grid yet): 100 boids cost about 7 ms per step in the debug build `dev.sh` runs, so the slider stops at 150.
- 3D `Vehicle::regenerate_basis` cannot turn to a forward parallel to the current `up` (the cross product is zero, so `side` and `up` become zero), as in the C++. Random 3D headings never hit it exactly; a straight-up or straight-down heading from the default basis would.
- The 3D ground composites (Capture the Flag, Pedestrians, Soccer) are the 2D runs lifted onto a flat plane; they have no 3D physics of their own.
- Silkroad world: one-way ledges are walls both ways; outline 0x08 (object stitch) is treated as open but Jangan has none, so it is untested; region-border walls from closed tiles are not built; bodies scrape walls and corners (avoidance looks only ahead); concave wall corners never tested; building floors are not drawn (the plaza walker seems to float over the terrain pits); the terrain mesh has a point every 80 game units while the walker's height uses the 20-unit tiles, so the body can sit a little above or below the drawn mesh; every step scans all 5887 walls (0.6-0.9 ms in debug) and `World` reloads (~0.1 s) on every select and reset; the demo's tests take ~14 s in debug.
- 3D Wall Following and Containment only know an axis-aligned box (`wall::Block`); there is no concave 3D room (the 2D L-room case). Navmesh walls are used only by obstacle avoidance ("3D Seek on Silkroad"), not by wall following or containment.
- Unproven causes, stated as "likely" in Findings: the 3D wanderer's larger speed overshoot, 3D path-following excursions, 3D Leader Following's fewer overlaps.

- Training Field: time inside the dps disk is now set by the caps, not the lurers' walk (73% /
  93% at 30 monsters with two lurers); lurer A delivers less than B in the one seed measured;
  the group heal never fired in the pinned run; scouting is random spots, so a lurer can zigzag
  on an empty side; the planners assume skill cooldowns stay as at planning time, and the stops
  planner keeps the fence only by candidate distance; every measurement is one seed (1) and two
  minutes; dps roam to 10 while the HP buffer reaches 8 from within 5; party giants are off
  (20,000 HP); a giant (2,000) in the disk still needs 75 s of focused fire; a monster's idle
  run uses the chase speed slider and a gait change cuts speed at once; no MP economy for the
  lurers' sessions; any new random draw shifts every pinned number (tests carry margins and the
  history; re-pin and say so); a test placing monsters must set their rarity to normal and
  freeze their gait when a position matters, and must track monsters by id across steps (a kill
  removes an entry); `trips` counts leaves and `sessions` returns; the party window's HP bars
  are assumed; the status lines are about 170 characters at 12 px, so they run off the right
  edge of a canvas narrower than about 1,250 px; labels pile up over the dps disk; two strong
  monsters arriving together (2,000 HP each) still hold both lurers for 70 s at strong cap 2;
  bodies graze rocks (avoidance looks only ahead, as on the plaza's walls); a lurer plans a
  strong monster while the disk holds one fewer than the cap, so two lurers can fill the disk to
  the cap plus one (with rocks: three 2,000 HP monsters from 80 s, both lurers held for minutes).

## Learning aids

Picture explainers ("eli5") published as private artifacts, in the app's colors, computed by a JS
copy of the integrator checked against the C++ oracles:
- One Turning Away: https://claude.ai/artifact/QK9f28JJ82R2h2beZPCL9U
- Seek: https://claude.ai/artifact/SkAWbZ4hPkWgMkEhEv5Jie
- Steering Family Tree (all 17 behaviors, how they derive from and combine each other):
  https://claude.ai/artifact/7rv3jpV4NSQYiYPwga9nWy
