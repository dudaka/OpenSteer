// Oracle: OpenSteer's xxxsteerForSeek / xxxsteerForFlee in 3D (targets off the ground plane), on its
// own SimpleVehicle, run headless. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/seek_flee_3d oracle/seek_flee_3d.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp \
//     && /tmp/seek_flee_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_seek_flee_3d.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

// From rest at the origin facing +z; maxForce 1, maxSpeed 3. Prints step 1 and every 30 steps.
void run(const char* name, bool seek, const Vec3& point) {
    Agent v;
    v.reset();
    v.setMaxForce(1);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 300; ++step) {
        const Vec3 force = seek ? v.xxxsteerForSeek(point) : v.xxxsteerForFlee(point);
        v.applySteeringForce(force, dt);
        if (step == 1 || step % 30 == 0) {
            const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
            std::printf("%s %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                        name, step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z,
                        v.speed());
        }
    }
}

int main() {
    run("seek", true, Vec3(6, 2, -4));
    run("flee", false, Vec3(1, -1, 1));
}
