// Oracle: OpenSteer's xxxsteerForSeek / xxxsteerForFlee (the versions the plug-ins use) on its
// own SimpleVehicle, run headless. Build and run from the repo root:
//   c++ -std=c++11 -w -Iinclude -o /tmp/seek_flee oracle/seek_flee.cpp oracle/annotation_stubs.cpp \
//     src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/seek_flee
// Its output is pinned in crates/steer-core/tests/oracle_seek_flee.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

// From rest at the origin facing +z; maxForce 1, maxSpeed 3. Prints every 30 steps of 1/60 s.
void run(const char* name, bool seek, const Vec3& point) {
    Agent v;
    v.reset();
    v.setMaxForce(1);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 300; ++step) {
        const Vec3 force = seek ? v.xxxsteerForSeek(point) : v.xxxsteerForFlee(point);
        v.applySteeringForce(force, dt);
        if (step % 30 == 0)
            std::printf("%s %d %.6f %.6f %.6f %.6f %.6f\n", name, step, v.position().x,
                        v.position().z, v.forward().x, v.forward().z, v.speed());
    }
}

int main() {
    run("seek", true, Vec3(6, 0, -4));
    run("flee", false, Vec3(1, 0, 1));
}
