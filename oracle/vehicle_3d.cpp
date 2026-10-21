// Oracle: OpenSteer's own C++ SimpleVehicle in 3D, run headless. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/vehicle_3d oracle/vehicle_3d.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp \
//     && /tmp/vehicle_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_vehicle_3d.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Plain : SimpleVehicle {
    void update(const float, const float) {}
};

// As Boids.cpp: roll into turns.
struct Banker : SimpleVehicle {
    void update(const float, const float) {}
    void regenerateLocalSpace(const Vec3& newVelocity, const float elapsedTime) {
        regenerateLocalSpaceForBanking(newVelocity, elapsedTime);
    }
};

static void print(const char* tag, int step, const SimpleVehicle& v) {
    const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
    std::printf("%s %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                tag, step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed());
}

int main() {
    const float dt = 1.0f / 60.0f;

    // A constant push with a vertical part: turns and climbs.
    Plain a;
    a.reset();
    a.setSpeed(1.5f);
    a.setMaxForce(0.3f);
    a.setMaxSpeed(5);
    for (int step = 1; step <= 600; ++step) {
        a.applySteeringForce(Vec3(-2, 1.5f, -3), dt);
        if (step == 1 || step % 60 == 0) print("climb", step, a);
    }

    // Circling to its side with banking on.
    Banker b;
    b.reset();
    b.setSpeed(1);
    b.setMaxForce(1);
    b.setMaxSpeed(2);
    for (int step = 1; step <= 600; ++step) {
        b.applySteeringForce(b.side() * 0.8f + b.forward() * 0.1f, dt);
        if (step == 1 || step % 60 == 0) print("bank", step, b);
    }
}
