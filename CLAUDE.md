# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose of this checkout

This is the original OpenSteer C++ library (Craig Reynolds' steering behaviors, ported to glfw + CMake in 2019). It is used here as a **reference implementation** to learn from and re-implement in Rust (simulation) with a React frontend (one demo page per behavior). Treat the C++ code as read-only reference unless asked otherwise.

## Start here

- `PLAN.md`: the step plan with `[done]` marks, **Status** (where we are, what is next), **Open questions**, **Findings** (verified behavior facts with numbers), **Gaps and known issues**, and the layout of every crate and package. Read it before starting a step.
- Stage 1 (2D, Phases 0-6) and the 3D arena part of stage 2 (Phase 7 steps A, B, E: `steer-core-3d`, `steer-demos-3d`, `Scene3D`) are done. Next is Phase 7 C/D, the Silkroad world (terrain and navmesh walls); its research notes and proposed sub-steps are in `PLAN.md`. Settle the open questions with the user before starting it.
- The user is learning the behaviors as we go. **Hold each new behavior until the user says to continue**, and ask before opening a PR; they ask conceptual questions (formulas, what each line means, what is given vs calculated) before moving on. Answer from the code, with numbers checked by running it. Only when the user asks for a batch: one branch and PR per step, merged on green CI without holding, one summary at the end.

## Commands

```
scripts/dev.sh                                    # steerd + Vite; open http://localhost:5173; Ctrl-C stops both
cargo test                                        # all Rust tests; also regenerates the TS contract types
cargo test -p steer-core seek                     # one test by name filter
cargo test -p steer-core --test oracle_seek_flee  # one 2D C++ oracle comparison
cargo test -p steer-core-3d --test oracle_boids_3d  # one 3D C++ oracle comparison
cargo fmt --all && cargo clippy --all-targets -- -D warnings
npm run typecheck && npm run build
```

Oracles: each `oracle/*.cpp` has its build line at the top (`c++ ... oracle/<name>.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp`, plus `src/Obstacle.cpp` or the pathway sources when used). They compile OpenSteer's own vehicle code headless; `annotation_stubs.cpp` replaces the OpenGL annotation hooks. Build into the scratchpad, run, and generate the Rust test's tables from the printed lines (a small `uv run` script), never by hand. CI never builds them: their output is pinned in the Rust tests. Randomness: build with `-Drand=oracle_rand` (an LCG the Rust test replays). Chaotic scenes (flocks, crowds, soccer) and every 3D oracle (`oracle/*_3d.cpp`): build with `-ffp-contract=off`, since clang on arm64 fuses multiply-adds and Rust does not. 3D oracles print all 13 state values (position, forward, side, up, speed) and check step 1 as well as regular steps; their tests share `crates/steer-core-3d/tests/common/mod.rs` (`State`, `check` at 1e-4, `OracleRand`).

## The Rust + React port

- The project setup **follows `~/Workspace/silkroad/caravan` exactly** (Cargo workspace `crates/*`, npm workspaces `packages/*` + `apps/*`, axum WebSocket daemon, MessagePack frames, ts-rs generated contract in `packages/core/src/contract/`, Redux Toolkit store, React + Vite, three.js). When unsure how to set something up, read the matching caravan file and copy its pattern (e.g. `steerd/src/origin.rs` is caravan's `LoopbackGuard`).
- Port faithfully and prove it: `oracle/*.cpp` run OpenSteer's own C++ headless; their output is pinned in `crates/*/tests/oracle_*.rs` (tolerance 1e-4, values stored as f64). A difference is a bug in the port until proven otherwise. After adding an oracle test, break the port on purpose once (a change that really alters behavior) and confirm the test fails.
- Crates: `steer-core` (2D math, vehicle, behaviors; no tokio/serde/IO) <- `steer-demos` (one module per 2D demo, `Demo` enum, view models) <- `steerd` (daemon, port 7400). In parallel: `steer-core-3d` <- `steer-demos-3d` (also uses `steer-demos` for colors, legend, controls and view types) <- `steerd`. Depend only downward.
- 3D (Phase 7) lives next to the 2D crates, not in them: `steer-core-3d` is the same port on `Vec3` with a stored forward/side/up basis (right-handed, `side = forward x up`), checked against the C++ without flattening. Oracle checkpoints must include an odd step: a wrong handedness flips `side` and `up` every step and hides on even ones.
- Rendering is three.js. 2D: an orthographic top-down camera; Rust 2D `(x, y)` is drawn on the three.js ground plane as `(x, 0, y)`, so +x is right and +y is down the screen (the 2D `y` is the 3D `z`).
- 3D demos (`steer-demos-3d`, `Demo3`, ids `3d-...`, listed after the 2D ones) send `Frame3View` with `[x, y, z]` points (+y up, drawn as they are; vehicles carry `forward` and `up` so roll shows). Frames reach the browser as `SceneFrame` tagged `kind: "flat" | "space"`; `App` shows `Scene` (top-down) or `Scene3D` (perspective, `OrbitControls`, camera placed per demo from `view_size`, then follows `focus`). 3D demos take no clicks (use buttons, e.g. "Add a rock"). They share the 2D color meanings, legend and controls (`steer_demos::{color, legend, controls}`); a height stem (`color::EDGE`) drops each point of interest to y = 0. OpenSteer's ground-bound composites are shown in 3D by `steer_demos_3d::ground::Ground`, which lifts the oracle-checked 2D run (no re-port).
- The frontend holds no steering logic: Rust computes every position, annotation, label text, legend entry and the camera focus into view models; the browser draws and sends typed calls (`demo.select|click|action|param`, `sim.play|pause|step|reset`). Never hand-write a wire type in TypeScript.
- Annotation colors have one meaning in every demo (`steer_demos::annotation::color`): **red = push** (steering force), **green = want** (desired velocity), **pink = have** (current velocity), **blue = change** (smoothed acceleration), yellow = target, gray = trail, neutral gray = text labels. Every frame carries a `legend`; a test fails if a demo draws a line color its legend does not explain. A new demo adds its colors to its legend, never reuses red/green for something else.
- A new demo: a module in `steer-demos` with `Default`, `update(dt)`, `view() -> FrameView`, optional `click`/`action`/`param`; add it to `Demo` (match arms) and the `DEMOS` catalog in `demo.rs`. A new 3D demo: the same in `steer-demos-3d` with `view() -> Frame3View`, added to `Demo3` and its `DEMOS` (id `3d-...`); annotation helpers in `steer_demos_3d::annotation` (`line`, `cross`, `sphere`, `ring`, `block_edges`, `stem`, `Trail`).
- Behaviors not in OpenSteer (Arrive, Leader Following, Wall Following, Containment, from Reynolds 1999) have no oracle: build them from the oracle-checked pieces, unit-test hand-computed cases, pin measured facts in demo tests, and still break the code once on purpose. When a design fails, trace the first failure (log the step where it goes wrong) before changing the design.

## Building the C++ reference demo

```
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/OpenSteerDemo
```

- Not yet built or run in this checkout (from `CMakeLists.txt`, unverified).
- Targets: `libopensteer` (static lib from `src/` + `include/OpenSteer/`) and `OpenSteerDemo` (OpenGL app + all `plugins/*.cpp`). glfw is vendored in `third-party/glfw`.
- The cppunit tests in `test/` are **not wired into CMake**; there is no test target.
- `src/terrain.zip` holds map data for the MapDrive plugin.

## Architecture (C++ reference)

- **Vehicle model** — `AbstractVehicle.h` -> `LocalSpace.h` (position + forward/side/up basis) -> `SimpleVehicle` (mass, speed, maxForce, maxSpeed). The core integrator is `SimpleVehicle::applySteeringForce` in `src/SimpleVehicle.cpp`: adjust raw force (low-speed turning limit), truncate to maxForce, divide by mass, smooth acceleration (blend rate `clip(9*dt, 0.15, 0.4)`), Euler-integrate velocity (truncate to maxSpeed) and position, then realign forward with velocity. `SimpleVehicle` is abstract (pure virtual `update`): an oracle subclasses it with an empty `update`.
- **Steering behaviors** — all live in `include/OpenSteer/SteerLibrary.h` as `SteerLibraryMixin<Super>`, a template mixin layered on top of a vehicle (`SimpleVehicle` = mixins over `LocalSpace`). 13 behaviors: seek, flee, pursuit, evasion, target speed, wander, obstacle avoidance, path following, stay on path, separation, alignment, cohesion, avoid neighbors. Each `steerFor*` / `steerTo*` returns a force vector; callers combine them (weighted sum or priority "return early if nonzero") and pass the result to `applySteeringForce`.
- **Supporting geometry** — `Obstacle.{h,cpp}` (sphere/box/plane obstacles, each implements `steerToAvoid`), path types `PolylineSegmentedPathwaySingleRadius` etc. (`mapPointToPath`, `mapPathDistanceToPoint`, used by path following), `Proximity.h` (`BruteForceProximityDatabase` and `LQProximityDatabase`, a grid-bin spatial index wrapping `src/lq.c`) for neighbor queries.
- **Demo plug-ins** — each `plugins/*.cpp` defines a `PlugIn` subclass (`open/update/redraw/close/allVehicles`) registered via a global singleton instance (pattern documented at the top of `PlugIn.h`). Plug-ins are the best place to see how behaviors are combined and tuned:
  - `OneTurning` (constant force, integrator only), `LowSpeedTurn`, `MultiplePursuit` (wander + pursuit), `Boids` (separation/alignment/cohesion + obstacles + proximity DB), `PedestriansWalkingAnEight` / `Pedestrian` (path following + obstacle and neighbor avoidance), `CaptureTheFlag` (seek/evade/avoid, braking), `Soccer`, `MapDrive` (large, terrain-map based driving).
- `OpenSteerDemo.cpp`, `Draw.cpp`, `Camera.cpp`, `Clock.cpp` are the OpenGL demo host (rendering, camera, timing); they are not needed to understand the behaviors.

## Gotchas when porting

- `steerForFlee` and `steerForEvasion` in `SteerLibrary.h` are broken (`position - target` without `()`, `this->menace`); they only compile because the templates are never instantiated. Plugins use `xxxsteerForSeek` / `xxxsteerForFlee` instead, which truncate desired velocity to `maxSpeed` — port those versions (done: `steer::seek`, `steer::flee`).
- Plain `steerForSeek` does **not** scale desired velocity to maxSpeed (it relies on later force truncation). `steerForPursuit` calls this plain version.
- The low-speed rule in `adjustRawSteeringForce` (below `0.2 * maxSpeed`, bend the force toward forward) shapes everything slow: it makes seek orbit its target instead of stopping. Test any "comes to rest" claim (Arrive) with it on.
- `steerForWander` returns a pure lateral force (`side*WanderSide + up*WanderUp`) driven by `scalarRandomWalk`; it is meant to be added to a forward force, not used alone. Its randomness is `frandom01() = rand() / RAND_MAX`, so an oracle must share the random draws.
- `steerForPursuit` uses a 3x3 table of time factors (quarry ahead/aside/behind x parallel/perpendicular/anti-parallel) to estimate intercept time.
- Most demos run on the XZ ground plane (`setYtoZero`), so a 2D port maps C++ `(x, z)` to `(x, y)`; C++ side is about `(-z, x)` = `Vec2::perp`, but not exactly: see the next point.
- Bit-level fidelity matters in chaotic scenes; a 1-ulp difference grows past 1e-4. Two such causes found by tracing to the first differing bit: C++ `pow(float, int)` is the double `pow` (port: `f64::powf`), and OpenSteer keeps `side`/`up` as state rebuilt by `regenerateOrthonormalBasisUF`, so `side` is `normalize(cross(forward, up))` with `up` drifting from 1 (port: `Vehicle::up`, `regenerate_basis`; never assign `forward` directly). When an oracle drifts, trace both sides every step and compare bits before touching the tolerance.
- 3D specifics: `SphereObstacle` intersects in the vehicle's local space (`localize_position`; the 2D port's world offsets differ only bit-level). `regenerate_basis` with a new forward parallel to `up` makes `side` and `up` zero (as the C++); avoid exactly vertical headings from the default basis. The Boids plug-in really is 3D (banking, spherical wrap-around at 50); CaptureTheFlag, Pedestrian(s) and Soccer flatten every force (`setYtoZero`). Banking rolls `side`, so a sideways push gains a vertical part.
- Scenario oracles can miss whole branches: the pursuit runs reach 5 of the 9 time-factor cells, so a separate test sets up all nine. When a mutation passes, find out which branch the scenario never visits and test it directly.

## Tooling gotchas (each cost time once)

- Do not edit Rust with sed/python string replacement after `cargo fmt` has run: the formatted text no longer matches and the edit silently does nothing. Read the file and use the Edit tool on the real text; check the edit applied. A `uv run` replace script must `assert s.count(old) == 1` for every replacement, so a mismatch fails loudly (it may already have written earlier files: re-check).
- Clippy on this toolchain flags `x % n != 0` (`manual_is_multiple_of`): write `!x.is_multiple_of(n)`. A shared `tests/common/mod.rs` needs `#![allow(dead_code)]` (each test file compiles it and uses only part of it).
- `dev.sh` runs a debug build: keep per-step work small enough for 60 steps a second (100 3D boids without a grid take ~7 ms per step).
- steerd (7400) and Vite (5173) have fixed ports, so a second copy cannot run beside the user's `dev.sh`; check views through tests, typecheck and build, and let the user look in their browser.
- three.js: a `LineSegments` whose points change every frame needs `frustumCulled = false` (its bounding sphere is computed once and goes stale). WebGL lines are always 1 px. `CSS2DRenderer` overwrites the element's `transform`; offset labels with `CSS2DObject.center`.
- Vite hot reload does not reliably re-run the lazily loaded scene: reload the page. After restarting `steerd`, wait for "connected" before clicking (buttons are disabled while reconnecting).
- Testing `dev.sh`: a backgrounded script ignores SIGINT; run it under a pseudo-terminal (`script -q /dev/null scripts/dev.sh`). macOS bash is 3.2 (no `wait -n`).
- Python `websockets` replaces a custom `Host` header; test Host/Origin handling with raw `curl` upgrade requests.
- `gh` on this fork: always pass `-R dudaka/OpenSteer`, or it targets the upstream `meshula/OpenSteer`.
- The user may have `scripts/dev.sh` running: do not kill it; ask them to restart it to pick up Rust changes.

## Git workflow

- Remotes: `origin` = the user's fork `dudaka/OpenSteer` (default branch `main`), `upstream` = `meshula/OpenSteer`. `origin/glfw_port` came with the fork; it is not ours.
- One branch per feature, made from `main` (the project skills in `.claude/skills/` only exist on branches made from `main`). Commit and push there, never straight to `main`. CI runs only on pushes to `main` and on pull requests.
- When the feature is done and the user agrees: open a PR, wait for CI to be green, merge with a merge commit, then delete the feature branch locally and on `origin`.

## The Silkroad world (Phase 7 C/D, next)

- Shared by `~/Workspace/silkroad/caravan` and `~/Workspace/silkroad/sro-polyanya` (read them read-only). The world is 2.5D: heightfield regions (1920 units, 97x97 heights) plus objects with stacked floors; Y is up; nothing flies.
- Decided: copy `caravan/crates/caravan-navmesh` (Rust, only `thiserror`) into this repo with a source note; do not add a path or git dependency. Game data lives in `~/Workspace/silkroad/Data` (577 MB); prebuilt terrain assets in `~/.caravan/map`. Tests that need the data skip when it is missing, so CI stays green.
- `caravan-navmesh` answers heights only (`Region::height_at`, `surface_heights`); walls exist as edge flags but there is no wall, nearest-point or path query. `sro-polyanya` (C++ over HTTP on 5577) has paths and line of sight, but slow (~20 s for long queries).

## Project skills (`.claude/skills/`, sources in `THIRD_PARTY.md`)

Use them while building and learning, but this file and the user's global rules win on any conflict (small incremental steps, no emojis, no overengineering).
- Rust: `rust-router` routes a Rust question to the right `m01`-`m15` skill (ownership, error handling, concurrency, type-driven design, performance, anti-patterns); `coding-guidelines` for style; `domain-web` for axum/tokio in `steerd`. `unsafe` is forbidden in this workspace.
- three.js: `threejs-debug-profiler` for blank canvas, resize and render bugs (its dev-only `window.__THREE_GAME_DIAGNOSTICS__` hook is wired in `scene/Scene.tsx` and `scene/Scene3D.tsx`); `threejs-qa-release` for canvas-pixel inspection and visual checks; `threejs-gameplay-systems` for camera, loop and input patterns; `threejs-aaa-graphics-builder` only if the user asks for richer 3D visuals (e.g. Silkroad terrain).
- Do not use `threejs-game-director` (it drives a whole-game production pass) or the asset generators (`threejs-3d/image/audio-generator`, paid APIs) unless the user asks.
- The user-level `eli5` plugin (`/eli5 <topic>`) makes a picture explainer page; previous ones are listed in `PLAN.md` (Learning aids).
