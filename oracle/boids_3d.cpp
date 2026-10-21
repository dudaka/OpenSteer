// Oracle: the Boids plug-in in 3D as OpenSteer has it (plugins/Boids.cpp): Boid::reset's random
// heading and position, steerToFlock's radii, angles and weights, banking
// (regenerateLocalSpaceForBanking) and the spherical wrap-around at radius 50, run headless.
// 30 boids; each updates in turn and sees the whole flock (no proximity database: the behaviors
// filter by distance themselves). No obstacles. rand() is the LCG of oracle/wander_3d.cpp.
// Flocking amplifies 1-ulp differences, so build without fused multiply-add. Build and run from
// the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Drand=oracle_rand -Iinclude -o /tmp/boids_3d \
//     oracle/boids_3d.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/boids_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_boids_3d.rs.
#include <cstdio>
#include <cstdlib>
#include <vector>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

static unsigned int lcg = 1;

// Same sequence as the Rust tests' `OracleRand`.
extern "C" int oracle_rand(void) {
    lcg = lcg * 1103515245u + 12345u;
    return (int)(lcg & 0x7fffffffu);
}

struct Boid : SimpleVehicle {
    void update(const float, const float) {}
    void regenerateLocalSpace(const Vec3& newVelocity, const float elapsedTime) {
        regenerateLocalSpaceForBanking(newVelocity, elapsedTime);
    }
    // Boid::reset without the proximity token.
    void resetBoid() {
        SimpleVehicle::reset();
        setMaxForce(27);
        setMaxSpeed(9);
        setSpeed(maxSpeed() * 0.3f);
        regenerateOrthonormalBasisUF(RandomUnitVector());
        setPosition(RandomVectorInUnitRadiusSphere() * 20);
    }
    // Boid::steerToFlock without obstacles.
    Vec3 steerToFlock(const AVGroup& flock) {
        const Vec3 separation = steerForSeparation(5.0f, -0.707f, flock);
        const Vec3 alignment = steerForAlignment(7.5f, 0.7f, flock);
        const Vec3 cohesion = steerForCohesion(9.0f, -0.15f, flock);
        return separation * 12.0f + alignment * 8.0f + cohesion * 8.0f;
    }
};

int main() {
    const int count = 30;
    std::vector<Boid> boids(count);
    AVGroup flock;
    for (int i = 0; i < count; ++i) {
        boids[i].resetBoid();
        flock.push_back(&boids[i]);
    }
    const float dt = 1.0f / 60.0f;
    int wraps = 0;
    for (int step = 0; step <= 600; ++step) {
        if (step > 0) {
            for (Boid& b : boids) {
                b.applySteeringForce(b.steerToFlock(flock), dt);
                if (b.position().length() > 50) {
                    b.setPosition(b.position().sphericalWrapAround(Vec3::zero, 50));
                    ++wraps;
                }
            }
        }
        if (step == 0 || step == 1 || step % 100 == 0) {
            for (int i = 0; i < count; i += 7) {
                const Boid& v = boids[i];
                const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
                std::printf("boid %d %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                            step, i, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z,
                            v.speed());
            }
            std::printf("wraps %d %d\n", step, wraps);
        }
    }
}
