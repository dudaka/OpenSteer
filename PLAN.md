# Rust + React re-implementation plan

Goal: re-implement OpenSteer steering behaviors in Rust, with a React page demonstrating each
behavior. Stage 1 is 2D. Each step is small and ends with a check.

Stage 2 (Phase 7, planned 2026-10-21): 3D, first in an empty arena, then in the Silkroad world
that `~/Workspace/silkroad/caravan` and `sro-polyanya` share. The 2D version stays alongside.

## Status (2026-10-21)

- Done: Phase 0 (skeleton), Phase 1 (vehicle + One Turning), Phase 2 steps 8-9 (Seek and Flee,
  Arrive with braking), demo controls, demo selection, click input, per-demo legend, parameter
  sliders.
- Done 2026-10-20 at the user's request: **every demo, steps 10-20**, one PR per step (#11-#19).
  Next: the user reviews the demos in the browser (none of steps 9-20 were viewed yet) and the
  "Open questions"; return to holding each behavior until they understand it.
- 2026-10-21: Phase 6 (Reynolds 1999 behaviors not in OpenSteer), in the user's order: step 21
  Leader Following (#20), Wall Following (#21) and Containment (#22) merged: Phase 6 complete.
- 2026-10-21: Phase 7 (3D) planned with the user (see below); working on step 24 (3D vector and
  vehicle) merged (#23); 3D seek and flee merged (#24); 3D wander and target speed merged
  (#25); 3D view merged (#26); 3D pursuit and evasion merged (#27); 3D obstacle avoidance
  merged (#28). From step 30 on, at the user's request, the remaining 3D demos run as a batch
  (one PR per step, merged on green CI); the user checks them all at the end.
- Tests: 228 Rust tests (20 C++ oracles: `oracle_vehicle_3d`, `oracle_seek_flee_3d`, `oracle_wander_3d`, `oracle_pursuit_3d`, `oracle_obstacles_3d`, `oracle_path_following_3d`, `oracle_boids_3d`, `oracle_neighbors_3d`, `oracle_one_turning`, `oracle_seek_flee`, `oracle_wander`, `oracle_pursuit`, `oracle_obstacles`, `oracle_pathway`, `oracle_path_following`, `oracle_boids`, `oracle_neighbors`, `oracle_capture_the_flag`, `oracle_pedestrians`, `oracle_soccer`).

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
                        npm ci, typecheck, build
scripts/dev.sh          builds steerd, runs it + Vite; Ctrl-C (or either exiting) stops both

crates/steer-core       L1 pure 2D math (Vec2, util), Vehicle + apply_steering_force, steer::{seek,
                        flee}. No tokio, no serde, no IO.
crates/steer-demos      L2 one module per demo: world setup, update(dt), click/action input and its
                        FrameView (serde + ts-rs). Demo enum dispatch, catalog, annotation helpers
                        and the shared color meanings (annotation::color). No tokio.
crates/steerd           L3 daemon on 127.0.0.1:7400 (caravand uses 7300): axum 0.8 `/ws` only,
                        tokio, rmp-serde. contract.rs: Request { id, call }, Call (tag "method" /
                        content "params"), Reply (tag "type" / content "value"), HostFrame (tag "t").
                        sim.rs: one Sim (demo + running) owned by the sim task, fixed 1/60 s steps,
                        commands in over mpsc, latest frame out over a watch channel.
                        origin.rs: caravan's LoopbackGuard (Host + Origin check, 403 otherwise).
                        Does NOT serve apps/web/dist yet: browse through Vite (scripts/dev.sh).

packages/core           @opensteer/core: MessagePack transport (@msgpack/msgpack) over a
                        WebSocket Link with reconnect, Host class with typed calls, Redux
                        Toolkit store whose reducers only copy what steerd sent, generated
                        contract types in src/contract/ (committed, never hand-written).
apps/web                @opensteer/web: React 19 + Vite + react-redux; Vite proxies /ws to
                        127.0.0.1:7400. three.js (as caravan), lazily loaded, orthographic
                        top-down camera on the XZ ground plane, so 2.5D/3D later only changes the
                        camera and adds height. Sidebar (demos), controls, actions, legend overlay.
```

Rules carried over from caravan:
- A crate depends only on crates below it (core <- demos <- steerd).
- **Frontend is dumb**: all steering math and annotations (velocity, force, predicted target,
  neighbor radius, legend, label text, camera focus) are computed in Rust into view models. The
  browser draws them and sends typed commands. Allowed client-side: visual interpolation between
  frames, and turning a click into a ground point (input handling).
- Contract types are generated by `ts-rs` on `cargo test`; CI fails if they differ.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run typecheck`
  green before a commit. Errors: `thiserror` in libs, `anyhow` only in `steerd`. Logging: `tracing`.
- Pin exact npm versions (caravan pins exact versions).

Commands:
```
scripts/dev.sh                          # steerd + Vite together, open http://localhost:5173; Ctrl-C stops both
cargo test                              # all Rust tests, regenerates contract types
cargo test -p steer-core seek           # one test by name filter
cargo test -p steer-core --test oracle_seek_flee   # one oracle comparison
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
6. [done] `Vehicle { position, forward, speed, mass, radius, max_force, max_speed, smoothed_acceleration }` (C++ keeps forward + speed, velocity = forward * speed) and `apply_steering_force(force, dt)` ported from `applySteeringForce` incl. `adjustRawSteeringForce`. Curvature and smoothed position (annotation-only) not ported. -> verify: property tests + `tests/oracle_one_turning.rs` matches the C++ `SimpleVehicle` (`oracle/one_turning.cpp`) to 1e-4 over 600 steps; a real mutation (clamp floor 0.15 -> 0.2) fails at step 60.
7. [done] Demo "One Turning Away" (`plugins/OneTurning.cpp`): constant force (-2, -3), trail, velocity/acceleration lines, the push drawn as a 4.5-unit direction under them, speed and start labels, camera follows the vehicle (Rust sends `focus`). -> verify: numeric match via the oracle; the C++ OpenSteerDemo app itself was never built or run, so there is no side-by-side visual comparison.

## Phase 2 - single-target behaviors (`include/OpenSteer/SteerLibrary.h`)

8. [done] Seek and Flee (port `xxxsteerForSeek/Flee`; `tests/oracle_seek_flee.rs` vs `oracle/seek_flee.cpp`, 1e-4 over 300 steps). Demo "Seek and Flee": click sets the target, Seek/Flee buttons, want (green) and push (red) drawn at true length (max_speed 3, so pink is also true length and red = green - pink closes a triangle).
9. [done] Arrive (Reynolds 1999, not in OpenSteer, so no oracle): `steer::arrive(v, target, slowing_radius)`, desired speed `max_speed * min(d / r, 1)`. Plus OpenSteer's `applyBrakingForce` as `Vehicle::apply_braking_force` (CaptureTheFlag, Soccer use it). Third mode in Seek and Flee, a Brake toggle (brakes inside the radius), sliders for slowing radius (1-15, 9) and braking rate (0.25-4, 0.75), radius circle. Tests pin the Findings cases.
10. [done] Wander (`steer::wander`, `util::scalar_random_walk`) and `steer::target_speed`. Random source injected as `FnMut() -> f32`; `oracle/wander.cpp` builds every file with `-Drand=oracle_rand` (an LCG) and `tests/oracle_wander.rs` replays it (two draws per call: side, then the unused up), 1e-4 over 600 steps; a 12 -> 11 walk-speed mutation fails at step 60. Only `WanderSide` is ported (up has no effect on the ground). Demo "Wander": `MpWanderer` (max_force 5, max_speed 3, push = forward + 3 * wander), buttons Forward push / Target speed, sliders wander weight and target speed, seeded `fastrand` so reset repeats the walk. Demos share `controls::{button, slider, Range}`.
11. [done] Pursuit and Evasion: `steer::intercept` (the 9-case table, returns target, factor and case name), `steer::pursuit` (plain seek of it), `steer::evasion` (plain flee; the C++ does not compile, ported as intended), `Vehicle::predict_future_position`. `oracle/pursuit.cpp` (pursuer vs the LCG wanderer; evader vs pursuer with a repaired copy of `steerForEvasion`), 1e-4 over 600 steps; mutations (factor 4 -> 3.5, no evasion cap) fail. Demo "Pursuit and Evasion": `MpPlugIn` (wanderer + 1-30 pursuers on a 20-30 ring, restart on touch, slider), aims drawn yellow instead of the C++ 9 case colors (red/green are reserved), the nearest pursuer labeled with its case and factor; mode "Pursuit vs evasion" (one hunter, evader max speed 2).
- verify each: unit test on force direction + oracle where OpenSteer has the behavior + demo page.

## Phase 3 - environment

12. [done] `steer_core::obstacle`: `Circle` (`SphereObstacle` on the ground, `SeenFrom` outside/inside/both), `intersect_path`, `nearest_intersection`, `avoid_obstacles` (`steerToAvoidObstacles`). `oracle/obstacles.cpp`: LCG wanderer in a container with three rocks, avoidance wins when nonzero (Pedestrian's priority); 1e-4 over 1800 steps incl. the count of avoiding steps (222). Mutations: no vehicle-radius widening fails at step 360; "inside a solid obstacle" distance 0 -> 1 was missed by the oracle, now caught by a unit test. Demo "Obstacle Avoidance": the oracle scene, click adds a rock, "Remove added rocks", slider min time to collision, look-ahead corridor and hit point (teal), obstacles brown.
13. [done] `steer_core::pathway::Pathway` (`PolylineSegmentedPathwaySingleRadius`): `map_point` (on-path point, tangent, outside, path distance: `mapPointToPath` + `mapPointToPathDistance`), `point_at` (`mapPathDistanceToPoint`), `length`, open or cyclic. Verified by a C++ oracle instead of the cppunit tests (those are not wired up): `oracle/pathway.cpp` / `tests/oracle_pathway.rs`, 12 points and 9 distances on an open and a cyclic path, incl. corner ties and out-of-range distances; mutations (`<` -> `<=` tie, no cyclic wrap) fail.
14. [done] `steer::follow_path` / `stay_on_path` (+ `*_target` for annotation; plain seek). `oracle/path_following.cpp`: the eight's path (y set to 0), one pedestrian `forward + 0.5 * path force`, directed with endpoint flips and stay-on-path, 1e-4 over 3600 steps; mutations (no right-way check; stay threshold 0 -> 0.5) fail. Demo "Path Following": 1-50 pedestrians (slider) on the eight with `Pedestrian::reset`'s random start, toggles "Follow in a direction" and "Wander" (both on, as the plug-in), pedestrian 0 shows its 3 s look-ahead, seek target and push.

## Phase 4 - groups

15. [done] `steer_core::proximity`: `brute_force` and `Grid` (square bins, `move_point` = `updateForNewPosition`), both return sorted indices strictly inside the radius. Tests: grid equals brute force for 200 queries at 4 radii and after 100 moves; a too-small cell range fails.
16. [done] `steer_core::flock`: `in_neighborhood`, `separation`, `alignment`, `cohesion` over an iterator of the other vehicles. `oracle/boids.cpp`: 12 boids with the plug-in's constants, sequential update, circular wrap-around, all 12 pinned every 100 steps for 600 steps; built with `-ffp-contract=off`. Mutations (`powi` back, 1/d separation, min neighborhood 2 radii) fail. Demo "Boids": 10-300 boids (100) in a radius-50 wrap-around circle, sliders for count and the three weights (12/8/8), grid kept current during the pass (a test: bit-identical to the whole flock over 600 steps), boid 0 shows its flockmates and push. Each frame now carries `view_size` (the camera's width in world units); earlier demos were cut off at 20.
17. [done] `steer_core::neighbors`: `nearest_approach_time`, `approach_positions`, `avoid_close_neighbors`, `threat` (earliest predicted approach within 2 radii, and the side), `avoid_neighbors`. `oracle/neighbors.cpp`: 8 pedestrians crossing a circle (Pedestrian's priority: forward + 10 * avoidance, else forward + seek), all pinned every 150 steps over 900 incl. the avoiding count; mutations (danger 2 -> 1.5 radii, no close-neighbor push) fail. Demo "Pedestrians Crossing": 2-30 on a circle walk to the opposite point and back, toggle "Avoid neighbors", count of overlapping steps, pedestrian 0's predicted nearest approach and push.

## Phase 5 - composite demos (optional)

18. [done] Demo "Capture the Flag" (`steer_demos::CaptureTheFlag`): the plug-in's seeker (`clearPathToGoal`, `XXXsteerToEvadeAllDefenders`, look-ahead 0.9/2 s, `limitMaxDeviationAngle` 0.707), 4 pursuing enemies, 40 random rocks (add/remove buttons), braking after a tag or goal and a restart 4 s later. `oracle/capture_the_flag.cpp` copies the plug-in logic with a fixed layout; `crates/steer-demos/tests/oracle_capture_the_flag.rs` pins two rounds (seeker evading for 20 s; seeker tagged at ~11 s, then braking) every 60 steps; mutations (evade behind-weight, corridor width, enemy braking) fail.
19. [done] Demo "Pedestrians" (`steer_demos::Pedestrians`): `Pedestrian.cpp`'s path and two rocks, 1-200 pedestrians (100), `determineCombinedSteering` (obstacle avoidance 6 s, else neighbor avoidance x 10, else wander + 0.5 x directed path following; both avoidances skipped 10% of steps), neighbors from the grid within 12. `step(dt, random01)` takes the random source. `oracle/pedestrians.cpp` (fixed layout of 10, LCG draws in the plug-in's order) pinned every 180 steps over 1800 incl. the avoidance counts; mutations (leak 0.2, one wander draw) fail. Not ported: the plug-in's tilted `RectangleObstacle` (see Gaps).
20. [done] Demo "Soccer" (`steer_demos::Soccer`, "Michael's Simple Soccer"): 8 v 8 on their spots, kick on touch, neighbor avoidance (1 s) against the players created earlier, seek the ball / get behind it / go home, a braking, bouncing ball, goals restart it. Deterministic (the plug-in's random draws are overwritten). `oracle/soccer.cpp` + `crates/steer-demos/tests/oracle_soccer.rs`: a minute, ball, score and two players every 120 steps; bit-identical traces over 3600 steps after the `side` fix below; mutations (exact-perp side, avoid everyone, no ball braking) fail.

## Phase 7 - 3D (stage 2)

Decided with the user 2026-10-21: keep the 2D crates and demos; add 3D alongside. Arena first,
then the world. The four 1999 behaviors get full 3D versions. The world's navmesh code is copied
into this repo (not a path or git dependency), so CI builds it; tests needing the game data
(`~/Workspace/silkroad/Data`, 577 MB) skip when it is missing, as caravan's do.

Research (2026-10-21, caravan and sro-polyanya read-only):
- The Silkroad world is 2.5D: per 1920-unit region a 97x97 height grid over 96x96 tiles of 20,
  plus object meshes (buildings, bridges) with stacked floors. Y is up. Characters walk on
  surfaces; nothing flies.
- `caravan/crates/caravan-navmesh` (Rust, depends only on `thiserror`): `Navmesh::load`,
  `Region::height_at(x, z)`, `surface_heights`; region and object edges carry flags (blocked,
  one-way ledge, bridge). No point location, nearest point, wall queries or path finding
  (caravan deferred its wall model, D32).
- `sro-polyanya` (C++, HTTP on 127.0.0.1:5577): Polyanya paths and line of sight; long queries
  take ~20 s; no height or wall query in its API.
- Caravan's renderer: `PerspectiveCamera(45)` + `OrbitControls`, terrain meshes from
  `~/.caravan/map/<region>/` (built by `caravan-import`).

Steps (each held for the user, as before):
- A. 3D core, crate `steer-core-3d`, every OpenSteer behavior on `Vec3` with a real
  forward/side/up basis, checked against the C++ in 3D (no flattening).
  - 24. [done] `steer_core_3d::{Vec3, Vehicle}`: `Vec3` (cross, truncate, components,
    `set_y_to_zero`), the vehicle with stored `forward`/`side`/`up` (`regenerate_basis` =
    `regenerateOrthonormalBasisUF`), `apply_steering_force`, `apply_braking_force`, and
    `banking` (`regenerateLocalSpaceForBanking`). `oracle/vehicle_3d.cpp` +
    `crates/steer-core-3d/tests/oracle_vehicle_3d.rs`: a constant climbing push and a banking
    circle, 600 steps, all 13 values at step 1 and every 60. Mutations (left-handed side,
    banking blend 2 instead of 3) fail.
  - 25. [done] `steer_core_3d::steer::{seek, flee}` (`xxxsteerForSeek` / `xxxsteerForFlee`).
    `oracle/seek_flee_3d.cpp` + `oracle_seek_flee_3d.rs`: from rest toward (6, 2, -4) and away
    from (1, -1, 1), 300 steps, all 13 values at step 1 and every 30. Mutation (uncapped want)
    fails seek. The oracle tests share `tests/common/mod.rs` (state order, 1e-4 check).
  - 26. [done] `steer_core_3d::steer::{wander, WanderState, target_speed}`: `steerForWander` with
    both walks (`WanderSide`, then `WanderUp`: two draws per call) and `steerForTargetSpeed`.
    `oracle/wander_3d.cpp` + `oracle_wander_3d.rs`: MultiplePursuit's wanderer without
    `setYtoZero`, forward push and target speed 1.5, 600 steps; the test replays the LCG
    (`tests/common::OracleRand`). Mutation (up drawn before side) fails both.
  - 28. [done] `steer_core_3d::steer::{intercept, Intercept, pursuit, evasion}` and the demo
    "3D Pursuit and Evasion" (pursuers start on a 20-30 shell with random 3D headings).
    `oracle/pursuit_3d.cpp` + `oracle_pursuit_3d.rs`: a pursuer chasing the 3D wanderer, an
    evader and a hunter (both vehicles checked, 600 steps), plus every cell of the 3x3 time-factor
    table (the two runs only reach 5 of 9: a "behind, parallel" change passed them and fails the
    cells test). Mutation ("ahead, parallel" 4 -> 2) fails all three.
  - 29. [done] `steer_core_3d::obstacle::{Sphere, SeenFrom, intersect_path,
    nearest_intersection, avoid_obstacles}` and `Vehicle::localize_position`; the intersection is
    computed in the vehicle's local space as the C++ (the 2D port used world offsets). Demo
    "3D Obstacle Avoidance" (container sphere 15 with three rocks at different heights, Add a
    rock / Remove added rocks, min time slider; spheres drawn as three great circles, the
    look-ahead as four lines along the side and up edges). `oracle/obstacles_3d.cpp` +
    `oracle_obstacles_3d.rs`: 1800 steps, all 13 values and the avoiding count at step 1 and
    every 90. Mutation (no vehicle radius in the widening) fails; world offsets instead of local
    space pass (bit-level only, as in 2D).
  - 30. [done] `steer_core_3d::pathway::Pathway` (the radius is a tube) and
    `steer::{stay_on_path(_target), follow_path(_target)}`. Demo "3D Path Following" on the
    figure eight raised into a roller coaster (heights 4 sin(i * 45 degrees)), tube drawn as
    rings. `oracle/path_following_3d.cpp` + `oracle_path_following_3d.rs`: directed and stay on
    path, 3600 steps, not flattened. Mutation (flat distance: a band, not a tube) fails both.
  - 31. [done] `steer_core_3d::flock::{in_neighborhood, separation, alignment, cohesion}`,
    `Vec3::spherical_wrap_around`, `util::{random_vector_in_unit_sphere, random_unit_vector}`.
    Demo "3D Boids": the plug-in as OpenSteer has it (banking, world sphere 50, 10-150 boids,
    weight sliders). `oracle/boids_3d.cpp` + `oracle_boids_3d.rs`: 30 boids from the plug-in's
    random reset (LCG replayed), 600 steps, every 7th boid and the wrap count; matches (chaotic,
    built with `-ffp-contract=off`). Mutation (banking off) fails.
  - 32. [done] `steer_core_3d::neighbors::{nearest_approach_time, approach_positions,
    avoid_close_neighbors, threat, Threat, avoid_neighbors}` (sideways is the stored `side`).
    Demo "3D Crossing": walkers on a Fibonacci sphere walk to the opposite point and back.
    `oracle/neighbors_3d.cpp` + `oracle_neighbors_3d.rs`: 12 walkers, 900 steps, every 5th walker
    and the avoiding count; matches. Mutation (the slower one gives way) fails.
  - 33. [done] The composites in the 3D view: "3D Capture the Flag (ground)", "3D Pedestrians
    (ground)", "3D Soccer (ground)" (`steer_demos_3d::ground::Ground`). OpenSteer's plug-ins
    flatten every force (`setYtoZero`), so they are not re-ported: the oracle-checked 2D runs are
    lifted onto the ground plane ((x, y) -> (x, 0, y), up straight up). A test checks the 3D view
    is the 2D run lifted. These move onto Silkroad terrain in step C.
  - Then the 1999 behaviors in full 3D (step E).
- B. 3D view: a perspective, orbiting camera in the web app; 3D demos in an empty arena.
  Brought forward on 2026-10-21 (before the remaining behaviors) so each 3D behavior gets its
  demo as it is ported.
  - 27. [done, viewed by the user 2026-10-21] Crate `steer-demos-3d` (`Frame3View` and friends,
    `Demo3`, catalog after the 2D demos); steerd sends `SceneFrame::{Flat, Space}`; web
    `Scene3D` (PerspectiveCamera 45, OrbitControls without pan, grid at y = 0, vehicles as a
    triangle with a mast along `up`, labels). Demos: "3D One Turning" (climbing turn / banking
    circle, both reproduce `oracle_vehicle_3d`), "3D Seek and Flee" (reproduces
    `oracle_seek_flee_3d`; Move target hops between four heights), "3D Wander" (forward or
    target speed, wander weight, banking toggle).
- C. The world: copy `caravan-navmesh` in; load a few regions (e.g. around Jangan); ground
  vehicles stand on the surface; draw the terrain.
- D. Walls from the navmesh's blocked edges: obstacle avoidance, containment and wall following
  against real walls.
- E. Arrive, Leader Following, Wall Following and Containment in full 3D (3D rooms, wall
  surfaces, flying leaders).

## Each demo page

three.js view, play/pause/step/reset, parameter sliders, toggles for each annotation. Demos are listed
in a sidebar, mirroring OpenSteerDemo's plug-in selection.

- [done] Play/pause/step/reset: calls `sim.play|pause|step|reset`; frames carry `running`. Step = pause + one 1/60 s step, as OpenSteerDemo. A paused sim sends no frames.
- [done] Demo selection: sidebar -> `demo.select` (unknown id refused with an error), `steer_demos::Demo` enum dispatch, frame carries the current `demo`.
- [done] Demo input: `demo.click {x, y}` (scene raycasts the click onto the ground) and `demo.action {id}` for the buttons each frame lists in `actions`.
- [done] Legend: each frame lists `legend` (color, meaning); one meaning per color across demos (`annotation::color`); a test fails if a demo draws a color its legend does not explain. Labels are neutral gray.
- [done] Parameter sliders: each frame lists `params` (id, label, min, max, step, value); `demo.param {id, value}` sets one; the demo clamps to the range. Shown next to the demo's buttons.
- Annotation toggles: when a demo needs them.

## Phase 6 - Reynolds 1999 behaviors not in OpenSteer (no C++ oracle)

Built from the ported, oracle-checked pieces; tests pin hand-computed cases and measured facts.

21. [done] Demo "Leader Following" (`steer_core::leader`, `steer_demos::LeaderFollowing`): followers `arrive` (slowing radius 3) at a point `behind` the leader plus weighted `separation` (2 units, rear cone excluded) from the others and the leader; inside the leader's path (4 ahead, leader radius + a follower diameter to each side) they instead want max speed sideways out of it. Leader: the Wander demo's push (max speed 1.5), followers max speed 3. Sliders: followers 1-20 (8), distance behind 1-8 (3), separation weight 0-5 (2). Mutation (step aside the wrong way) fails a core and a demo test.
22. [done] Demo "Wall Following" (`steer_core::wall`, `steer_demos::WallFollowing`): `Wall` is a closed counter-clockwise polygon (`edges`, `distance`, `offset` with miter corners: edges stay exactly the offset away, a right-angle outer corner is offset x sqrt 2); `wall_path` makes the offset outline a cyclic `Pathway` (reversed for clockwise, so direction is always +1 and the cyclic `fmod` quirk at negative distances never applies). Wall following is then the ported `follow_path`: three walkers (max speed 2, max force 8) steer with `forward + steerToFollowPath` around an L-shaped building, band radius 0.5. Buttons: counter-clockwise / clockwise; sliders: offset 0.5-6 (2), look-ahead 0.25-3 s (1). Mutation (inward normal) fails 3 core tests and the demo measurement.
23. [done] Demo "Containment" (`steer_core::containment`, `steer_demos::Containment`): `Wall::contains` (even-odd ray crossing); three probes `radius + look_ahead * speed` out (ahead, 45 degrees to each side); while the ahead probe is outside, a `max_force` push along `side` toward the side probe with more room (signed distance to the wall, +side on a tie). Five wanderers (the Obstacle Avoidance demo's wander, max speed 3, max force 5) in an L-shaped room; while the push is on they also brake (`apply_braking_force`). Sliders: wanderers 1-20 (5), look-ahead 0.25-3 s (1), brake while containing 0-4 (4). Label: escapes, wanderer-steps outside, furthest out. Mutation (turn toward less room) fails 4 core tests and both demo measurements.

## Open questions (for the user, after the batch)

- Arrive: keep the faithful "orbits unless braking is tuned" result, or add a non-OpenSteer stop rule?
- Obstacles: keep the faithful container widening (body crosses the arena wall by ~0.5), or shrink containers?
- Pedestrians: port the tilted rectangle as a 2D wall obstacle?

## Findings (verified, with numbers)

- **Seek never stops, it orbits.** From rest at the origin to target (-3, 4) with max_force 1, max_speed 3: after 30 s it circles at 0.459 from the target, speed 0.583. The C++ prints the same digits. Cause, proven by switching it off: the low-speed rule (`adjustRawSteeringForce`, below 0.2 * max_speed bends the force toward forward) removes the braking component; with the rule off the vehicle stops on the target (distance 0.000). This is why Arrive exists.
- **OpenSteer's seek slows down by itself.** `xxxsteerForSeek` truncates the offset to max_speed, so inside max_speed units the desired speed equals the distance. Plain `steerForSeek` does not.
- **Arrive alone does not stop either** (rule on, same setup): r=3 equals OpenSteer's seek to every digit; r=5/9/15 orbit at speed 0.583 (just under 0.2 * max_speed = 0.6), distance 0.5-0.64 at 30 s. With the rule off every radius stops (r=9 fastest, step 612). r >= max_speed^2 / max_force = 9 never asks for more than max_force.
- **Braking can stop Arrive, but only if it never passes the target.** Braking inside the radius: r=9 rate 0.75 and r=5 rate 1.5 come to rest and stay (checked to 120 s). r=5 rate 0.75 gets within 0.0003, creeps past (target cos +1 -> -1 at ~22.5 s), and the low-speed rule turns the pull back into a forward push: speed doubles about every second. Rate 3 holds speed at exactly max_force / rate = 0.333 (full forward push vs full braking) and wanders.
- **Wandering runs faster than the target speed.** Target 1.5, mean speed 10-60 s: weight 0 -> 1.500, 1 -> 1.521, 3 -> 1.669, 5 -> 1.901 (the C++ shows the same). Cause, proven by switching smoothing off (weight 3 drops to 1.517): the smoothed acceleration lags the turning side vector, so part of the sideways force points forward; the rest is Euler integration lengthening the velocity each turn step.
- **An equally fast pursuer never catches a wanderer.** Oracle: pursuer from (8, -6) behind `MpWanderer` (both max_speed 3) is still 3.04 behind after 10 s. MultiplePursuit's pursuers catch the wanderer only when it turns toward them.
- **Plain flee slows down near the menace.** `steerForEvasion` uses plain `steerForFlee`: desired speed = distance from the predicted position, so the evader (max 2) runs at 0.38-0.85 once the hunter is within ~1-2 units (oracle steps 120-600) instead of escaping.
- **A container lets the body cross its wall.** `SphereObstacle` widens every obstacle by the vehicle radius, also one seen from inside, so the arena (radius 15) is tested at 15.5: over 2 min the center reaches 15.02 (206 of 7200 steps beyond 15). Rocks keep a full body width (min clearance 0.5003). Shrinking the container by the radius instead keeps the center within 14.17 (not ported: not OpenSteer).
- **Mapping quirks (pathway).** Ties between segments go to the earlier one (strict `<`), so the corner (10, 6) maps to segment 1. On a cyclic path a negative distance is not wrapped: `fmod(-3, L) = -3`, then clamped to 0 (the start).
- **Only directed following without wander keeps pedestrians inside the path.** Ten pedestrians, 60 s after 10 s settling, worst distance outside: directed alone -0.092 (never out); directed + wander (the plug-in's default) 2.326; stay-on-path alone 1.939 (no direction, so they walk off the open ends: 1487 of 2682 outside steps within 1 of an end).
- **The eight's endpoints are 3.06 apart**, so with radius 2 a directed pedestrian at one end can flip direction twice in quick succession (oracle: -1 at step 2520, +1 again by 2700).
- **The low-speed rule's `pow` must be computed in double.** C++ `pow(range, 20)` with a float and an int promotes to the double `pow`; the port used `f32::powi`, 1 ulp apart at times. Earlier oracles hid it under 1e-4; flocking amplifies it (boid 0 off by 2e-3 at step 200). Traced to the first bit difference (step 12, boid 6, low-speed regime); with `f64::powf` and the oracle built with `-ffp-contract=off` (clang on arm64 fuses multiply-adds by default) the traces are bit-identical over 7200 boid-steps. Oracles of chaotic scenes need `-ffp-contract=off`.
- **A neighbor grid must follow the boids during the pass.** A boid that wraps around jumps ~100 units; a grid built at the start of the pass missed it (step 284: boid 40 wrapped to 5.52 from boid 49). C++ updates each boid's proximity token after it moves; the port does the same (`Grid::move_point`).
- **In perpendicular encounters the faster vehicle gives way**, not the slower as the C++ comment says: the code tests `threat->speed() <= speed()`. The port follows the code.
- **Neighbor avoidance reduces but does not prevent overlaps.** 8 crossing pedestrians, 60 s: 847 steps with bodies overlapping without avoidance, 57 with it (it reacts only to the earliest threat, and an overlap gets only a sideways push).
- **Capture the Flag is about even.** Random rounds over 10 min: 32 ended, 17 reached home, 15 tagged, mean round 14.7 s.
- **Pedestrians can start inside a rock.** The rocks sit on the path and `Pedestrian::reset` does not avoid them: 6 of 100 start inside (the C++ does the same). In 60 s nobody else enters one; this was proven by elimination (leak-through off: 600 vs 609 inside-steps; neighbor avoidance off: 623), then by counting starts (6) and new entries (0).
- **`side` is state in OpenSteer, not a function of `forward`.** `regenerateOrthonormalBasisUF` sets `side = normalize(cross(forward, up))` from the previous `up`, then `up = cross(side, forward)`; on the ground `up` stays (0, u, 0) with u drifting from 1 (e.g. 1.000002). The port's exact `perp` was a few ulps off; soccer drifted from step 85 (first bit difference: player 4's avoidance push = side x steer) to 1.6e-4 by step 1080. With `Vehicle::up` + `regenerate_basis` the soccer trace is bit-identical over 3600 steps; all earlier oracles still pass. Pedestrians Crossing's overlap count with avoidance moved from 57 to 87 (chaotic).
- **Soccer players avoid only earlier players.** Each `Player` copies `m_AllPlayers` by value when built, so team A's first player avoids nobody and team B's last avoids 15.
- **Separation is 0/0 for two vehicles on the same spot.** `steerForSeparation` divides each offset by its squared length. Leader Following with separation weight 0: followers pile onto the point behind and at step 1510 two are bit-identical, so separation (times 0) turned NaN. `follow_leader` skips separation at weight 0; the ported `separation` is unchanged.
- **Leader Following: separation spreads followers but they still overlap.** 8 followers, 60 s: weight 2 -> mean 3.88 from the point behind, 672 of 3600 steps with an overlap, 3.1% of follower-steps stepping aside; weight 0 -> 3.21, 3524 steps, 0.14%.
- **Wall following overshoots every 90-degree corner by about 1.4.** Offset 2, band +-0.5, look-ahead 1 s, 15-60 s: distance from the wall min 0.585, mean 2.306, max 3.385 (clockwise 0.814 / 2.348 / 3.405). Located by logging: the minimum is just after the inner corner (walkers run past the outline's turn at (1, 2) to x = -0.41, 0.59 from the wall edge x = -1), the maximum just after the outer corners (x = 9.37 vs the outline at 8). Path following only reacts once the predicted position leaves the band. Look-ahead 3 s cuts corners and rides closer (0.527 / 1.485 / 3.476); offset 4: 2.642 / 4.406 / 5.790.
- **Containment took four designs; each failure was traced to its first escape.** (1) The paper's push (the sideways part of the nearest edge's inward normal): past a convex corner two edges tie for nearest and their pushes alternate (+5 y / -5 y) and cancel. (2) The full inward normal (turn and brake): the wanderer slows to 0.58, under the low-speed rule's 0.6, which bends the inward push forward into the wall (rule off: 4389 -> 58 outside steps). (3) Sideways toward the nearest point: heading into a corner, that direction is nearly anti-parallel to forward, so its sideways sign flips with heading noise. (4) Side probes, turn toward the roomier one: the choice holds (turning toward room makes that side roomier); what is left is physical: at speed 3 and max force 5 the turning radius (~1.8) is too wide to get out of a corner.
- **Containment needs braking at corners.** 5 wanderers, 60 s (escapes / wanderer-steps outside / furthest out): look-ahead 1 s, no braking 4 / 364 / 2.05 (all come back); brake 1: 3 / 86 / 0.64; brake 4: 0 / 0 / 0. Look-ahead 0.5 s: no braking 32 / 2029 / 0.94 (one still out at the end), brake 4: 1 / 31 / 0.13. Look-ahead 2 s, brake 4: 1 / 93 / 0.70. Only the default (1 s, 4) kept everyone in.
- **A banking vehicle circling to its side sinks.** Push `0.8 side + 0.1 forward`, banking on (C++ and port): y = -3.30 after 10 s. Rolling into the turn tilts `side` down (side.y = -0.025 at 10 s), so the sideways push gains a downward part; Boids counter this with their sphere constraint.
- **3D seek orbits like 2D seek, in a tilted plane.** From rest, max force 1, max speed 3, after 30 s: target (-3, 0, 4) on the ground gives distance 0.459, speed 0.583, y = 0 throughout (the 2D digits exactly); target (6, 2, -4) gives 0.460 and 0.583 on a circle tilted 18.4 degrees (plane normal (-0.316, 0.949, 0); y between 1.85 and 2.14).
- **The 3D wanderer overshoots its target speed more than the 2D one, and drifts in height.** Target speed 1.5, mean speed 10-60 s (2D in brackets): weight 0 -> 1.500 (1.500), 3 -> 1.827 (1.669), 5 -> 2.191 (1.901). Over 60 s at weight 3 its height ranges -5.02..7.78, and its `up` has rolled to y = 0.446 without banking (the basis keeps `up` as close as it can, but climbing turns still roll it). Cause of the larger overshoot not yet proven (the 2D cause was smoothing lag of the sideways force; 3D has two lateral parts).
- **3D pursuit: 20 catches a minute; the evader is caught at 3.87 s.** 30 pursuers on a 20-30 shell around the 3D wanderer catch it 20 times in 60 s. Pursuit vs evasion (evader max speed 2, hunter 3, from (-5, -2, -5)): the hunter touches the evader at step 232, since plain flee slows near the menace (the 2D finding). The scenario runs only use 5 of the 9 time-factor cells (pursuit: ahead/aside parallel; evasion adds ahead/aside perpendicular and behind perpendicular).
- **3D obstacle avoidance repeats the 2D container finding.** Wanderer in a container sphere 15 with three rocks, 2 min: the center reaches 15.022 from the container's center (widened by the vehicle radius, as in 2D: 15.02); rock clearance stays at least 0.860 (body radius 0.5); avoiding 1142 of 7200 steps. Local-space versus world-offset intersection differs only below 1e-4 over 1800 steps.
- **On a 3D path even directed following leaves the tube.** Roller-coaster eight, radius 2, 10 walkers, worst distance outside over 60 s after 10 s: directed 0.602 (2D: never out), directed + wander 1.705 (2D 2.326), stay on path 0.915 (2D 1.939), stay on path + wander 3.349. Likely the vertical curves (not proven).
- **3D Boids fly in all three dimensions.** 100 boids, 10 s: 93 wrap-arounds at the radius-50 sphere, heights -36.4..39.3, mean speed 8.59 of 9. The 30-boid oracle run matches the C++ over 600 steps (24 wraps).
- **3D crossing: avoidance cuts overlaps by 78%.** 12 walkers through the center of a radius-10 sphere, 60 s: 570 steps with bodies overlapping without avoidance, 125 with it (2D, 8 walkers on a circle: 847 and 87).
- **A wrong-handed basis hides on even steps.** With `side = up x forward` both `side` and `up` change sign every step, so checkpoints every 60 steps matched the C++ for a constant push; only the banking run (which uses `up`) caught it. The 3D oracle now also checks step 1.
- **Flee has no "far enough"**: it runs away forever at max_speed.
- **One Turning:** speed dips from 1.5 to 0.963 at 2.80 s (0.832 at 4.25 s with the low-speed rule off), reaches max speed 5 at 20.7 s. The smoothed acceleration matches the capped push except during the first ~0.5 s (smoothing, 15% per step at 60 Hz) and ~2.5-4.5 s (low-speed rule).
- **Integrator per step**: low-speed bend -> cap at max_force -> /mass -> blend 15% (clip(9*dt, 0.15, 0.4)) into smoothed acceleration -> velocity += acc*dt, cap at max_speed -> position += velocity*dt -> forward = velocity direction.

## Gaps and known issues

- `steerd` does not serve the built web app (`apps/web/dist`); only Vite does (`scripts/dev.sh`).
- One shared sim for all browser clients: one current demo and one running state.
- 3D Boids looks at every other boid (no 3D proximity grid yet): 100 boids cost about 7 ms per step in the debug build `dev.sh` runs, so the slider stops at 150.
- steerd's port (7400) and Vite's (5173) are fixed, so a second copy cannot run beside the user's `dev.sh`; the user checks views in their own browser.
- Annotation lines are 1 px (WebGL `LineSegments`); three's `Line2` fat lines if too faint.
- Sidebar is a fixed 180 px column (not phone-friendly); labels overlap when the car is near the target; the page has no favicon (404 in the console).
- No annotation toggles yet. Sliders exist (`params` / `demo.param`).
- Arrive, Brake and the sliders were not viewed in the browser (the running `dev.sh` predated them).
- The One Turning push line change (PR #8) is covered by a unit test but was not viewed in the browser.
- The C++ OpenSteerDemo app (CMake build) was never built here; its cppunit tests are not wired into CMake.
- Pedestrians lacks the plug-in's third obstacle, a 7x7 `RectangleObstacle` tilted 45 degrees (seen from both sides): on the ground it is a wall at x = 20, z -3..4, but its steer hint has a vertical part that shrinks the in-plane push after `setYtoZero`, so a faithful 2D port needs a dedicated wall obstacle.
- `SphereObstacle` intersection in C++ uses the vehicle's local coordinates (built from `side`/`up`); the port uses world offsets. Same values to 1e-4 in the oracles; bit-level only.
- Pedestrians and Boids take neighbors in index order; OpenSteer's LQ database returns them in bin order, which matters only for which of several equal threats is answered first.

## Learning aids

Picture explainers ("eli5") published as private artifacts, in the app's colors, computed by a JS
copy of the integrator checked against the C++ oracles:
- One Turning Away: https://claude.ai/artifact/QK9f28JJ82R2h2beZPCL9U
- Seek: https://claude.ai/artifact/SkAWbZ4hPkWgMkEhEv5Jie
- Steering Family Tree (all 17 behaviors, how they derive from and combine each other):
  https://claude.ai/artifact/7rv3jpV4NSQYiYPwga9nWy
