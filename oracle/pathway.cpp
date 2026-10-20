// Oracle: OpenSteer's PolylineSegmentedPathwaySingleRadius mappings (mapPointToPath,
// mapPointToPathDistance, mapPathDistanceToPoint), open and cyclic, on the ground plane. Build
// and run from the repo root:
//   c++ -std=c++11 -w -Iinclude -o /tmp/pathway oracle/pathway.cpp \
//     src/PolylineSegmentedPathwaySingleRadius.cpp src/PolylineSegmentedPath.cpp \
//     src/SegmentedPathway.cpp src/SegmentedPath.cpp src/Pathway.cpp src/Path.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp && /tmp/pathway
// Its output is pinned in crates/steer-core/tests/oracle_pathway.rs.
#include <cstdio>
#include "OpenSteer/PolylineSegmentedPathwaySingleRadius.h"

using namespace OpenSteer;

void run(const char* name, bool cyclic) {
    const Vec3 points[] = {Vec3(0, 0, 0), Vec3(10, 0, 0), Vec3(10, 0, 6), Vec3(4, 0, 10)};
    PolylineSegmentedPathwaySingleRadius path(4, points, 1.5f, cyclic);
    const float queries[][2] = {{5, 1},  {5, -3}, {11, -1}, {12, 3},  {9, 1},   {-2, -2},
                                {7, 9},  {2, 5},  {4, 10},  {3, 11}, {10, 6},  {6, 3}};
    for (const auto& q : queries) {
        const Vec3 point(q[0], 0, q[1]);
        Vec3 tangent;
        float outside;
        const Vec3 onPath = path.mapPointToPath(point, tangent, outside);
        std::printf("%s point %g %g %.6f %.6f %.6f %.6f %.6f %.6f\n", name, q[0], q[1], onPath.x,
                    onPath.z, tangent.x, tangent.z, outside, path.mapPointToPathDistance(point));
    }
    const float distances[] = {-3, 0, 4, 10, 13, 20, 23.2111f, 25, 40};
    for (float d : distances) {
        const Vec3 p = path.mapPathDistanceToPoint(d);
        std::printf("%s distance %g %.6f %.6f\n", name, d, p.x, p.z);
    }
    std::printf("%s length %.6f\n", name, path.length());
}

int main() {
    run("open", false);
    run("cyclic", true);
}
