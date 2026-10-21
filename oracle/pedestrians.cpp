// Oracle: OpenSteer's Pedestrian plug-in steering (plugins/Pedestrian.cpp,
// determineCombinedSteering and the endpoint flips) on its path and two sphere obstacles, run
// headless with 10 pedestrians in a fixed layout. Left out: the tilted rectangle obstacle (3D),
// the random placement and the LQ database (neighbors are every pedestrian within 12, in index
// order). rand() is the LCG of oracle/wander.cpp, drawn in the plug-in's order: obstacle leak,
// neighbor leak, wander side, wander up. Built without fused multiply-add, as Rust computes.
// Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Drand=oracle_rand -Iinclude -o /tmp/pedestrians \
//     oracle/pedestrians.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Obstacle.cpp \
//     src/PolylineSegmentedPathwaySingleRadius.cpp src/PolylineSegmentedPath.cpp \
//     src/SegmentedPathway.cpp src/SegmentedPath.cpp src/Pathway.cpp src/Path.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/pedestrians
// Its output is pinned in crates/steer-demos/tests/oracle_pedestrians.rs.
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/Obstacle.h"
#include "OpenSteer/PolylineSegmentedPathwaySingleRadius.h"

using namespace OpenSteer;

static unsigned int lcg = 1;

// Same sequence as the Rust tests' `OracleRand`.
extern "C" int oracle_rand(void) {
    lcg = lcg * 1103515245u + 12345u;
    return (int)(lcg & 0x7fffffffu);
}

struct Pedestrian : SimpleVehicle {
    void update(const float, const float) {}
    int pathDirection;
};

const int count = 10;
Pedestrian crowd[count];
ObstacleGroup gObstacles;
SphereObstacle gObstacle1, gObstacle2;

// Pedestrian::determineCombinedSteering with brute-force neighbors.
Vec3 combinedSteering(Pedestrian& p, PolylineSegmentedPathwaySingleRadius& path, float dt,
                     int& obstacleSteps, int& neighborSteps) {
    Vec3 steeringForce = p.forward();
    const float leakThrough = 0.1f;
    Vec3 obstacleAvoidance;
    if (leakThrough < frandom01()) obstacleAvoidance = p.steerToAvoidObstacles(6, gObstacles);
    if (obstacleAvoidance != Vec3::zero) {
        steeringForce += obstacleAvoidance;
        ++obstacleSteps;
    } else {
        Vec3 collisionAvoidance;
        const float caLeadTime = 3;
        const float maxRadius = caLeadTime * p.maxSpeed() * 2;
        AVGroup neighbors;
        for (int j = 0; j < count; ++j)
            if ((crowd[j].position() - p.position()).lengthSquared() < maxRadius * maxRadius)
                neighbors.push_back(&crowd[j]);
        if (leakThrough < frandom01())
            collisionAvoidance = p.steerToAvoidNeighbors(caLeadTime, neighbors) * 10;
        if (collisionAvoidance != Vec3::zero) {
            steeringForce += collisionAvoidance;
            ++neighborSteps;
        } else {
            steeringForce += p.steerForWander(dt);
            steeringForce += p.steerToFollowPath(p.pathDirection, 3, path) * 0.5;
        }
    }
    return steeringForce.setYtoZero();
}

int main() {
    const float size = 30, top = 2 * size, gap = 1.2f * size, out = 2 * size, h = 0.5;
    const Vec3 points[7] = {Vec3(h + gap - out, 0, h + top - out), Vec3(h + gap, 0, h + top),
                            Vec3(h + gap + (top / 2), 0, h + top / 2), Vec3(h + gap, 0, h),
                            Vec3(h, 0, h), Vec3(h, 0, h + top), Vec3(h + gap, 0, h + top / 2)};
    PolylineSegmentedPathwaySingleRadius path(7, points, 2, false);
    gObstacle1.center = interpolate(0.2f, points[0], points[1]);
    gObstacle2.center = interpolate(0.5f, points[2], points[3]);
    gObstacle1.radius = 3;
    gObstacle2.radius = 5;
    gObstacles.push_back(&gObstacle1);
    gObstacles.push_back(&gObstacle2);

    // Pedestrian::reset's speeds; positions every 25 along the path, varied headings,
    // alternating directions.
    for (int i = 0; i < count; ++i) {
        Pedestrian& p = crowd[i];
        p.reset();
        p.setMaxSpeed(2.0);
        p.setMaxForce(8.0);
        p.setSpeed(0);
        p.setRadius(0.5);
        p.setPosition(path.mapPathDistanceToPoint(i * 25.0f));
        const float a = i * 1.1f;
        p.regenerateOrthonormalBasisUF(Vec3(std::cos(a), 0, std::sin(a)));
        p.pathDirection = (i % 2 == 0) ? 1 : -1;
    }
    const float dt = 1.0f / 60.0f;
    int obstacleSteps = 0, neighborSteps = 0;
    for (int step = 1; step <= 1800; ++step) {
        for (int i = 0; i < count; ++i) {
            Pedestrian& p = crowd[i];
            p.applySteeringForce(combinedSteering(p, path, dt, obstacleSteps, neighborSteps), dt);
            if (Vec3::distance(p.position(), points[0]) < path.radius()) p.pathDirection = +1;
            if (Vec3::distance(p.position(), points[6]) < path.radius()) p.pathDirection = -1;
        }
        if (step % 180 == 0)
            for (int i = 0; i < count; ++i)
                std::printf("pedestrians %d %d %.6f %.6f %.6f %.6f %.6f %d %d %d\n", step, i,
                            crowd[i].position().x, crowd[i].position().z, crowd[i].forward().x,
                            crowd[i].forward().z, crowd[i].speed(), crowd[i].pathDirection,
                            obstacleSteps, neighborSteps);
    }
}
