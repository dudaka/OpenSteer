// Oracle: OpenSteer's steerToAvoidNeighbors (unaligned collision avoidance, with
// steerToAvoidCloseNeighbors first), run headless: 8 pedestrians on a circle of radius 10 each
// walk to the opposite point. Steering as Pedestrian.cpp's priority: forward + 10 * avoidance
// when it is nonzero, otherwise forward + xxxsteerForSeek(target). Built without fused
// multiply-add, as Rust computes. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/neighbors oracle/neighbors.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp \
//     src/Color.cpp && /tmp/neighbors
// Its output is pinned in crates/steer-core/tests/oracle_neighbors.rs.
#include <cmath>
#include <cstdio>
#include <vector>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

int main() {
    const int count = 8;
    std::vector<Agent> agents(count);
    std::vector<Vec3> targets(count);
    AVGroup all;
    for (int i = 0; i < count; ++i) {
        Agent& a = agents[i];
        a.reset();
        a.setMaxSpeed(2);
        a.setMaxForce(8);
        a.setRadius(0.5f);
        a.setSpeed(1);
        const float angle = i * (6.2831853f / count);
        const Vec3 start(10 * std::cos(angle), 0, 10 * std::sin(angle));
        a.setPosition(start);
        targets[i] = -start;
        a.regenerateOrthonormalBasisUF((targets[i] - start).normalize());
        all.push_back(&a);
    }
    const float dt = 1.0f / 60.0f;
    int avoiding = 0;
    for (int step = 1; step <= 900; ++step) {
        for (int i = 0; i < count; ++i) {
            Agent& a = agents[i];
            const Vec3 avoidance = a.steerToAvoidNeighbors(3, all) * 10;
            Vec3 steer = a.forward();
            if (avoidance != Vec3::zero) {
                steer += avoidance;
                ++avoiding;
            } else {
                steer += a.xxxsteerForSeek(targets[i]);
            }
            a.applySteeringForce(steer, dt);
        }
        if (step % 150 == 0)
            for (int i = 0; i < count; ++i)
                std::printf("neighbors %d %d %.6f %.6f %.6f %.6f %.6f %d\n", step, i,
                            agents[i].position().x, agents[i].position().z,
                            agents[i].forward().x, agents[i].forward().z, agents[i].speed(),
                            avoiding);
    }
}
