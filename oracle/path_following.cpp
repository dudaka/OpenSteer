// Oracle: OpenSteer's steerToFollowPath (directed, flipping at the endpoints) and
// steerToStayOnPath on PedestriansWalkingAnEight's figure-eight path, run headless. One
// pedestrian steers with forward + 0.5 * path force (no obstacles, neighbors or wander). The
// path points' small y values are set to 0 (the port is 2D). Build and run from the repo root:
//   c++ -std=c++11 -w -Iinclude -o /tmp/path_following oracle/path_following.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/PolylineSegmentedPathwaySingleRadius.cpp \
//     src/PolylineSegmentedPath.cpp src/SegmentedPathway.cpp src/SegmentedPath.cpp src/Pathway.cpp \
//     src/Path.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/path_following
// Its output is pinned in crates/steer-core/tests/oracle_path_following.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/PolylineSegmentedPathwaySingleRadius.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

const Vec3 eight[16] = {
    Vec3(-12.678730011f, 0, 0.523285984993f), Vec3(-10.447640419f, 0, -3.44138407707f),
    Vec3(-5.88988399506f, 0, -4.1717581749f),  Vec3(0.941263973713f, 0, 0.350513994694f),
    Vec3(5.83484792709f, 0, 6.56243610382f),   Vec3(11.0144147873f, 0, 10.175157547f),
    Vec3(15.9621419907f, 0, 8.82364273071f),   Vec3(18.697883606f, 0, 2.42084693909f),
    Vec3(16.0552558899f, 0, -3.57153511047f),  Vec3(10.5450153351f, 0, -9.92683887482f),
    Vec3(5.88374519348f, 0, -8.51393127441f),  Vec3(3.17790007591f, 0, -2.33129906654f),
    Vec3(1.94371795654f, 0, 2.78656601906f),   Vec3(-1.04967498779f, 0, 5.57114219666f),
    Vec3(-7.58111476898f, 0, 6.13661909103f),  Vec3(-12.4111375809f, 0, 3.5670940876f)};

// Pedestrian::reset without the random parts: from rest at `start`, facing +z.
void run(const char* name, bool directed, const Vec3& start) {
    PolylineSegmentedPathwaySingleRadius path(16, eight, 2, false);
    Agent v;
    v.reset();
    v.setMaxSpeed(2);
    v.setMaxForce(8);
    v.setSpeed(0);
    v.setRadius(0.5f);
    v.setPosition(start);
    int direction = 1;
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 3600; ++step) {
        const Vec3 pathFollow = directed ? v.steerToFollowPath(direction, 3, path)
                                         : v.steerToStayOnPath(3, path);
        v.applySteeringForce((v.forward() + pathFollow * 0.5f).setYtoZero(), dt);
        if (directed) {
            if (Vec3::distance(v.position(), eight[0]) < path.radius()) direction = +1;
            if (Vec3::distance(v.position(), eight[15]) < path.radius()) direction = -1;
        }
        if (step % 180 == 0)
            std::printf("%s %d %.6f %.6f %.6f %.6f %.6f %d\n", name, step, v.position().x,
                        v.position().z, v.forward().x, v.forward().z, v.speed(), direction);
    }
}

int main() {
    run("follow", true, Vec3(-8, 0, -3));
    run("stay", false, Vec3(4, 0, 2));
}
