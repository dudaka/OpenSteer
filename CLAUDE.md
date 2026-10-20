# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose of this checkout

This is the original OpenSteer C++ library (Craig Reynolds' steering behaviors, ported to glfw + CMake in 2019). It is used here as a **reference implementation** to learn from and re-implement in Rust (simulation) with a React frontend (one demo page per behavior). Treat the C++ code as read-only reference unless asked otherwise.

## The Rust + React port

- Plan, layout, commands and progress: `PLAN.md`. Stage 1 is **2D only**; 2.5D/3D with terrain and navmesh comes later, after research.
- The project setup **follows `~/Workspace/silkroad/caravan` exactly** (Cargo workspace `crates/*`, npm workspaces `packages/*` + `apps/*`, axum WebSocket daemon, MessagePack frames, ts-rs generated contract in `packages/core/src/contract/`, Redux Toolkit store, React + Vite). When unsure how to set something up, read the matching caravan file and copy its pattern.
- Crates: `steer-core` (math, vehicle, behaviors; no tokio/serde/IO) <- `steer-demos` (one module per C++ plug-in, view models) <- `steerd` (daemon, port 7400). Depend only downward.
- The frontend holds no steering logic: Rust computes every position and annotation into view models; the browser draws and sends typed commands. Never hand-write a wire type in TypeScript.

## Building the C++ reference demo

```
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/OpenSteerDemo
```

- Targets: `libopensteer` (static lib from `src/` + `include/OpenSteer/`) and `OpenSteerDemo` (OpenGL app + all `plugins/*.cpp`). glfw is vendored in `third-party/glfw`.
- The cppunit tests in `test/` are **not wired into CMake**; there is no test target.
- `src/terrain.zip` holds map data for the MapDrive plugin.

## Architecture (C++ reference)

- **Vehicle model** — `AbstractVehicle.h` -> `LocalSpace.h` (position + forward/side/up basis) -> `SimpleVehicle` (mass, speed, maxForce, maxSpeed). The core integrator is `SimpleVehicle::applySteeringForce` in `src/SimpleVehicle.cpp`: adjust raw force (low-speed turning limit), truncate to maxForce, divide by mass, smooth acceleration (blend rate `clip(9*dt, 0.15, 0.4)`), Euler-integrate velocity (truncate to maxSpeed) and position, then realign forward with velocity.
- **Steering behaviors** — all live in `include/OpenSteer/SteerLibrary.h` as `SteerLibraryMixin<Super>`, a template mixin layered on top of a vehicle (`SimpleVehicle` = mixins over `LocalSpace`). Each `steerFor*` / `steerTo*` returns a force vector; callers combine them (weighted sum or priority "return early if nonzero") and pass the result to `applySteeringForce`.
- **Supporting geometry** — `Obstacle.{h,cpp}` (sphere/box/plane obstacles, each implements `steerToAvoid`), path types `PolylineSegmentedPathwaySingleRadius` etc. (`mapPointToPath`, `mapPathDistanceToPoint`, used by path following), `Proximity.h` (`BruteForceProximityDatabase` and `LQProximityDatabase`, a grid-bin spatial index wrapping `src/lq.c`) for neighbor queries.
- **Demo plug-ins** — each `plugins/*.cpp` defines a `PlugIn` subclass (`open/update/redraw/close/allVehicles`) registered via a global singleton instance (pattern documented at the top of `PlugIn.h`). Plug-ins are the best place to see how behaviors are combined and tuned:
  - `OneTurning` (constant force, integrator only), `LowSpeedTurn`, `MultiplePursuit` (wander + pursuit), `Boids` (separation/alignment/cohesion + obstacles + proximity DB), `PedestriansWalkingAnEight` / `Pedestrian` (path following + obstacle and neighbor avoidance), `CaptureTheFlag` (seek/evade/avoid, braking), `Soccer`, `MapDrive` (large, terrain-map based driving).
- `OpenSteerDemo.cpp`, `Draw.cpp`, `Camera.cpp`, `Clock.cpp` are the OpenGL demo host (rendering, camera, timing); they are not needed to understand the behaviors.

## Gotchas when porting

- `steerForFlee` and `steerForEvasion` in `SteerLibrary.h` are broken (`position - target` without `()`, `this->menace`); they only compile because the templates are never instantiated. Plugins use `xxxsteerForSeek` / `xxxsteerForFlee` instead, which truncate desired velocity to `maxSpeed` — port those versions.
- Plain `steerForSeek` does **not** scale desired velocity to maxSpeed (it relies on later force truncation).
- `steerForWander` returns a pure lateral force (`side*WanderSide + up*WanderUp`) driven by `scalarRandomWalk`; it is meant to be added to a forward force, not used alone.
- `steerForPursuit` uses a 3x3 table of time factors (quarry ahead/aside/behind x parallel/perpendicular/anti-parallel) to estimate intercept time.
- Most demos run on the XZ ground plane (`setYtoZero`), so a 2D port maps C++ `(x, z)` to `(x, y)`.
