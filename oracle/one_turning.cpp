// Oracle: OpenSteer's own C++ SimpleVehicle, run headless. Build and run from the repo root:
//   c++ -std=c++11 -w -Iinclude -o /tmp/one_turning oracle/one_turning.cpp oracle/annotation_stubs.cpp \
//     src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/one_turning
// Its output is pinned in crates/steer-core/tests/oracle_one_turning.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Turner : SimpleVehicle {
    void update(const float, const float) {}
};

int main() {
    Turner v;
    v.reset();
    v.setSpeed(1.5f);
    v.setMaxForce(0.3f);
    v.setMaxSpeed(5);
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 600; ++step) {
        v.applySteeringForce(Vec3(-2, 0, -3), dt);
        if (step % 60 == 0)
            std::printf("%d %.6f %.6f %.6f %.6f %.6f\n", step, v.position().x, v.position().z,
                        v.forward().x, v.forward().z, v.speed());
    }
}

