// Oracle: OpenSteer's steerToFollowPath (directed, flipping at the endpoints) and
// steerToStayOnPath in 3D: PedestriansWalkingAnEight's figure eight raised into a roller coaster
// (heights 4 sin(i * 45 degrees)), radius 2, so the path is a tube. One walker steers with
// forward + 0.5 * path force, not flattened (the plug-in flattens it). Build and run from the repo
// root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/path_following_3d \
//     oracle/path_following_3d.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp \
//     src/PolylineSegmentedPathwaySingleRadius.cpp src/PolylineSegmentedPath.cpp \
//     src/SegmentedPathway.cpp src/SegmentedPath.cpp src/Pathway.cpp src/Path.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/path_following_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_path_following_3d.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/PolylineSegmentedPathwaySingleRadius.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

const Vec3 eight[16] = {
    Vec3(-12.678730011f, 0.0f, 0.523285984993f),
    Vec3(-10.447640419f, 2.8f, -3.44138407707f),
    Vec3(-5.88988399506f, 4.0f, -4.1717581749f),
    Vec3(0.941263973713f, 2.8f, 0.350513994694f),
    Vec3(5.83484792709f, 0.0f, 6.56243610382f),
    Vec3(11.0144147873f, -2.8f, 10.175157547f),
    Vec3(15.9621419907f, -4.0f, 8.82364273071f),
    Vec3(18.697883606f, -2.8f, 2.42084693909f),
    Vec3(16.0552558899f, -0.0f, -3.57153511047f),
    Vec3(10.5450153351f, 2.8f, -9.92683887482f),
    Vec3(5.88374519348f, 4.0f, -8.51393127441f),
    Vec3(3.17790007591f, 2.8f, -2.33129906654f),
    Vec3(1.94371795654f, 0.0f, 2.78656601906f),
    Vec3(-1.04967498779f, -2.8f, 5.57114219666f),
    Vec3(-7.58111476898f, -4.0f, 6.13661909103f),
    Vec3(-12.4111375809f, -2.8f, 3.5670940876f)};

static void print(const char* tag, int step, const SimpleVehicle& v, int direction) {
    const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
    std::printf("%s %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %d\n",
                tag, step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed(),
                direction);
}

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
        v.applySteeringForce(v.forward() + pathFollow * 0.5f, dt);
        if (directed) {
            if (Vec3::distance(v.position(), eight[0]) < path.radius()) direction = +1;
            if (Vec3::distance(v.position(), eight[15]) < path.radius()) direction = -1;
        }
        if (step == 1 || step % 180 == 0) print(name, step, v, direction);
    }
}

int main() {
    run("follow", true, Vec3(-8, 1, -3));
    run("stay", false, Vec3(4, -1, 2));
}
