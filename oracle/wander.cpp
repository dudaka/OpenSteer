// Oracle: OpenSteer's steerForWander and steerForTargetSpeed on its own SimpleVehicle, run
// headless. rand() is replaced in every file by an LCG (-Drand=oracle_rand) so the Rust test can
// replay the same random draws. Build and run from the repo root:
//   c++ -std=c++11 -w -Drand=oracle_rand -Iinclude -o /tmp/wander oracle/wander.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp \
//     src/Color.cpp && /tmp/wander
// Its output is pinned in crates/steer-core/tests/oracle_wander.rs.
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

// MultiplePursuit's wanderer: from rest at the origin facing +z, maxForce 5, maxSpeed 3. The
// forward push is forward() (MpWanderer) or steerForTargetSpeed(1.5). Prints every 30 steps.
void run(const char* name, bool targetSpeed) {
    lcg = 1;
    Agent v;
    v.reset();
    v.setSpeed(0);
    v.setMaxForce(5);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 600; ++step) {
        const Vec3 wander2d = v.steerForWander(dt).setYtoZero();
        const Vec3 push = targetSpeed ? v.steerForTargetSpeed(1.5f) : v.forward();
        v.applySteeringForce(push + wander2d * 3, dt);
        if (step % 60 == 0)
            std::printf("%s %d %.6f %.6f %.6f %.6f %.6f\n", name, step, v.position().x,
                        v.position().z, v.forward().x, v.forward().z, v.speed());
    }
}

int main() {
    run("forward", false);
    run("target", true);
}
