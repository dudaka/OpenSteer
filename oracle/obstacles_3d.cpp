// Oracle: OpenSteer's steerToAvoidObstacles with SphereObstacles in 3D (no setYtoZero), run
// headless: a wanderer in a container sphere (seen from inside) with three solid rocks at
// different heights. As in Pedestrian.cpp, the avoidance force wins when it is nonzero. rand() is
// the LCG of oracle/wander_3d.cpp. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Drand=oracle_rand -Iinclude -o /tmp/obstacles_3d \
//     oracle/obstacles_3d.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Obstacle.cpp \
//     src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/obstacles_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_obstacles_3d.rs.
#include <cstdio>
#include <cstdlib>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/Obstacle.h"

using namespace OpenSteer;

static unsigned int lcg = 1;

// Same sequence as the Rust tests' `OracleRand`.
extern "C" int oracle_rand(void) {
    lcg = lcg * 1103515245u + 12345u;
    return (int)(lcg & 0x7fffffffu);
}

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

int main() {
    SphereObstacle arena(15, Vec3(0, 0, 0));
    arena.setSeenFrom(Obstacle::inside);
    SphereObstacle rock1(2, Vec3(0, 2, 6));
    SphereObstacle rock2(1.5f, Vec3(-6, -3, -2));
    SphereObstacle rock3(3, Vec3(7, 1, -6));
    ObstacleGroup obstacles;
    obstacles.push_back(&arena);
    obstacles.push_back(&rock1);
    obstacles.push_back(&rock2);
    obstacles.push_back(&rock3);

    // MpBase::reset: at rest at the origin facing +z, maxForce 5, maxSpeed 3.
    Agent v;
    v.reset();
    v.setSpeed(0);
    v.setMaxForce(5);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    int avoiding = 0;
    for (int step = 1; step <= 1800; ++step) {
        const Vec3 avoidance = v.steerToAvoidObstacles(1.0f, obstacles);
        Vec3 steer = avoidance;
        if (avoidance == Vec3::zero) {
            steer = v.forward() + v.steerForWander(dt) * 3;
        } else {
            ++avoiding;
        }
        v.applySteeringForce(steer, dt);
        if (step == 1 || step % 90 == 0) {
            const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
            std::printf("obstacles %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                        step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed());
            std::printf("avoiding %d %d\n", step, avoiding);
        }
    }
}
