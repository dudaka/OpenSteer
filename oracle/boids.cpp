// Oracle: OpenSteer's steerForSeparation / steerForAlignment / steerForCohesion with the Boids
// plug-in's weights, radii, angles, speeds and wrap-around, on the ground plane, run headless.
// 12 boids in a fixed layout; each updates in turn and sees the whole flock (no proximity
// database: the behaviors filter by distance themselves). No obstacles. Flocking amplifies 1-ulp
// differences, so build without fused multiply-add (-ffp-contract=off), as Rust computes. Build
// and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/boids oracle/boids.cpp oracle/annotation_stubs.cpp \
//     src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/boids
// Its output is pinned in crates/steer-core/tests/oracle_boids.rs.
#include <cmath>
#include <cstdio>
#include <vector>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Boid : SimpleVehicle {
    void update(const float, const float) {}

    // Boid::steerToFlock without obstacles.
    Vec3 steerToFlock(const AVGroup& flock) {
        const Vec3 separation = steerForSeparation(5.0f, -0.707f, flock);
        const Vec3 alignment = steerForAlignment(7.5f, 0.7f, flock);
        const Vec3 cohesion = steerForCohesion(9.0f, -0.15f, flock);
        return separation * 12.0f + alignment * 8.0f + cohesion * 8.0f;
    }
};

int main() {
    const int count = 12;
    std::vector<Boid> boids(count);
    AVGroup flock;
    for (int i = 0; i < count; ++i) {
        Boid& b = boids[i];
        b.reset();
        b.setMaxForce(27);
        b.setMaxSpeed(9);
        b.setSpeed(9 * 0.3f);
        const float a = i * 0.7f, r = 2 + i * 0.8f, h = i * 1.3f;
        b.regenerateOrthonormalBasisUF(Vec3(std::cos(h), 0, std::sin(h)));
        b.setPosition(Vec3(r * std::cos(a), 0, r * std::sin(a)));
        flock.push_back(&b);
    }
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 600; ++step) {
        for (Boid& b : boids) {
            b.applySteeringForce(b.steerToFlock(flock), dt);
            b.setPosition(b.position().sphericalWrapAround(Vec3::zero, 50));
        }
        if (step % 100 == 0)
            for (int i = 0; i < count; ++i)
                std::printf("boids %d %d %.6f %.6f %.6f %.6f %.6f\n", step, i,
                            boids[i].position().x, boids[i].position().z, boids[i].forward().x,
                            boids[i].forward().z, boids[i].speed());
    }
}
