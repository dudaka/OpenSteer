# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose of this checkout

This is the original OpenSteer C++ library (Craig Reynolds' steering behaviors, ported to glfw + CMake in 2019). It is used here as a **reference implementation** to learn from and re-implement in Rust (simulation) with a React frontend (one demo page per behavior). Treat the C++ code as read-only reference unless asked otherwise.

## Start here

- `PLAN.md`: the step plan with `[done]` marks, **Status** (where we are, what is next), **Findings** (verified behavior facts with numbers), **Gaps and known issues**, and the layout of every crate and package. Read it before starting a step.
- Stage 1 is **2D only**; 2.5D/3D with terrain and navmesh comes later, after research.
- The user is learning the behaviors as we go. **Hold each new behavior until the user says to continue**; they ask conceptual questions (formulas, what each line means, what is given vs calculated) before moving on. Answer from the code, with numbers checked by running it.

## Commands

```
scripts/dev.sh                                    # steerd + Vite; open http://localhost:5173; Ctrl-C stops both
cargo test                                        # all Rust tests; also regenerates the TS contract types
cargo test -p steer-core seek                     # one test by name filter
cargo test -p steer-core --test oracle_seek_flee  # one C++ oracle comparison
cargo fmt --all && cargo clippy --all-targets -- -D warnings
npm run typecheck && npm run build
```

Oracles: each `oracle/*.cpp` has its build line at the top (`c++ ... oracle/<name>.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp`). They compile OpenSteer's own vehicle code headless; `annotation_stubs.cpp` replaces the OpenGL annotation hooks.

## The Rust + React port

- The project setup **follows `~/Workspace/silkroad/caravan` exactly** (Cargo workspace `crates/*`, npm workspaces `packages/*` + `apps/*`, axum WebSocket daemon, MessagePack frames, ts-rs generated contract in `packages/core/src/contract/`, Redux Toolkit store, React + Vite, three.js). When unsure how to set something up, read the matching caravan file and copy its pattern (e.g. `steerd/src/origin.rs` is caravan's `LoopbackGuard`).
- Port faithfully and prove it: `oracle/*.cpp` run OpenSteer's own C++ headless; their output is pinned in `crates/*/tests/oracle_*.rs` (tolerance 1e-4, values stored as f64). A difference is a bug in the port until proven otherwise. After adding an oracle test, break the port on purpose once (a change that really alters behavior) and confirm the test fails.
- Crates: `steer-core` (math, vehicle, behaviors; no tokio/serde/IO) <- `steer-demos` (one module per demo, `Demo` enum, view models) <- `steerd` (daemon, port 7400). Depend only downward.
- Rendering is three.js with an orthographic top-down camera; Rust 2D `(x, y)` is drawn on the three.js ground plane as `(x, 0, y)`, so +x is right and +y is down the screen.
- The frontend holds no steering logic: Rust computes every position, annotation, label text, legend entry and the camera focus into view models; the browser draws and sends typed calls (`demo.select|click|action`, `sim.play|pause|step|reset`). Never hand-write a wire type in TypeScript.
- Annotation colors have one meaning in every demo (`steer_demos::annotation::color`): **red = push** (steering force), **green = want** (desired velocity), **pink = have** (current velocity), **blue = change** (smoothed acceleration), yellow = target, gray = trail, neutral gray = text labels. Every frame carries a `legend`; a test fails if a demo draws a line color its legend does not explain. A new demo adds its colors to its legend, never reuses red/green for something else.
- A new demo: a module in `steer-demos` with `Default`, `update(dt)`, `view() -> FrameView`, optional `click`/`action`; add it to `Demo` (match arms) and the `DEMOS` catalog in `demo.rs`.

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
- Most demos run on the XZ ground plane (`setYtoZero`), so a 2D port maps C++ `(x, z)` to `(x, y)`; C++ side `(-z, x)` is `Vec2::perp`.

## Tooling gotchas (each cost time once)

- Do not edit Rust with sed/python string replacement after `cargo fmt` has run: the formatted text no longer matches and the edit silently does nothing. Read the file and use the Edit tool on the real text; check the edit applied.
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

## Project skills (`.claude/skills/`, sources in `THIRD_PARTY.md`)

Use them while building and learning, but this file and the user's global rules win on any conflict (small incremental steps, no emojis, no overengineering).
- Rust: `rust-router` routes a Rust question to the right `m01`-`m15` skill (ownership, error handling, concurrency, type-driven design, performance, anti-patterns); `coding-guidelines` for style; `domain-web` for axum/tokio in `steerd`. `unsafe` is forbidden in this workspace.
- three.js: `threejs-debug-profiler` for blank canvas, resize and render bugs (its dev-only `window.__THREE_GAME_DIAGNOSTICS__` hook is wired in `scene/Scene.tsx`); `threejs-qa-release` for canvas-pixel inspection and visual checks; `threejs-gameplay-systems` for camera, loop and input patterns; `threejs-aaa-graphics-builder` only at the 2.5D/3D stage.
- Do not use `threejs-game-director` (it drives a whole-game production pass) or the asset generators (`threejs-3d/image/audio-generator`, paid APIs) unless the user asks.
- The user-level `eli5` plugin (`/eli5 <topic>`) makes a picture explainer page; previous ones are listed in `PLAN.md` (Learning aids).
