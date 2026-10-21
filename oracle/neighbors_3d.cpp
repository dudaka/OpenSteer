// Oracle: OpenSteer's steerToAvoidNeighbors (unaligned collision avoidance, with
// steerToAvoidCloseNeighbors first) in 3D, run headless: 12 walkers spread over a sphere of radius
// 10 (a Fibonacci spiral) each head for the opposite point, so every path crosses the center.
// Steering as Pedestrian.cpp's priority: forward + 10 * avoidance when it is nonzero, otherwise
// forward + xxxsteerForSeek(target). Built without fused multiply-add, as Rust computes. Build and
// run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/neighbors_3d oracle/neighbors_3d.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp \
//     src/Color.cpp && /tmp/neighbors_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_neighbors_3d.rs.
#include <cmath>
#include <cstdio>
#include <vector>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

int main() {
    const int count = 12;
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
        // Fibonacci sphere: heights evenly spaced, golden-angle turns.
        const float y = 1 - (i + 0.5f) * (2.0f / count);
        const float r = std::sqrt(1 - y * y);
        const float angle = i * 2.39996323f;
        const Vec3 start = Vec3(r * std::cos(angle), y, r * std::sin(angle)) * 10;
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
        if (step == 1 || step % 150 == 0) {
            for (int i = 0; i < count; i += 5) {
                const Agent& v = agents[i];
                const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
                std::printf("walker %d %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n",
                            step, i, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z,
                            v.speed());
            }
            std::printf("avoiding %d %d\n", step, avoiding);
        }
    }
}
