// Oracle: OpenSteer's steerForWander and steerForTargetSpeed in 3D (no setYtoZero, so the
// wanderer also drifts up and down) on its own SimpleVehicle, run headless. rand() is replaced in
// every file by an LCG (-Drand=oracle_rand) so the Rust test can replay the same random draws.
// Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Drand=oracle_rand -Iinclude -o /tmp/wander_3d \
//     oracle/wander_3d.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/wander_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_wander_3d.rs.
#include <cstdio>
#include <cstdlib>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

static unsigned int lcg = 1;

// Same sequence as the Rust test's `OracleRand`.
extern "C" int oracle_rand(void) {
    lcg = lcg * 1103515245u + 12345u;
    return (int)(lcg & 0x7fffffffu);
}

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

// MultiplePursuit's wanderer in 3D: from rest at the origin facing +z, maxForce 5, maxSpeed 3,
// force push + 3 * wander, the push being forward() or steerForTargetSpeed(1.5). Prints step 1
// and every 60 steps.
void run(const char* name, bool targetSpeed) {
    lcg = 1;
    Agent v;
    v.reset();
    v.setSpeed(0);
    v.setMaxForce(5);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 600; ++step) {
        const Vec3 wander = v.steerForWander(dt);
        const Vec3 push = targetSpeed ? v.steerForTargetSpeed(1.5f) : v.forward();
        v.applySteeringForce(push + wander * 3, dt);
        if (step == 1 || step % 60 == 0) {
            const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
            std::printf("%s %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                        name, step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z,
                        v.speed());
        }
    }
}

int main() {
    run("forward", false);
    run("target", true);
}
