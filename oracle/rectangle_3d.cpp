// Oracle: OpenSteer's RectangleObstacle (PlaneObstacle::findIntersectionWithVehiclePath with
// RectangleObstacle::xyPointInsideShape) and steerToAvoidObstacles, run headless in 3D.
// Part "hit": single vehicle poses (position, forward) against an upright 6x2 wall seen from
// outside, inside and both, and Pedestrian.cpp's tilted 7x7 rectangle (seen from both): whether
// the path meets it, the distance, the steer hint and the surface point. Each line prints its
// inputs. Part "walls": a vehicle seeking back and forth across the upright wall and the tilted
// rectangle, avoidance winning when nonzero (as in Pedestrian.cpp), 1800 steps. Build and run
// from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/rectangle_3d oracle/rectangle_3d.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Obstacle.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/rectangle_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_rectangle_3d.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/Obstacle.h"

using namespace OpenSteer;

struct Agent : SimpleVehicle {
    void update(const float, const float) {}
};

// side = forward x up, as the vehicles' basis.
static RectangleObstacle wall(AbstractObstacle::seenFromState seen) {
    return RectangleObstacle(6, 2, Vec3(-1, 0, 0), Vec3(0, 1, 0), Vec3(0, 0, 1), Vec3(0, 1, 4), seen);
}

static RectangleObstacle tilted() {
    return RectangleObstacle(7, 7, Vec3(0, 0, 1), Vec3(-1, 1, 0).normalize(), Vec3(1, 1, 0).normalize(),
                             Vec3(3, 0, -3), Obstacle::both);
}

static void hit(const char* name, const RectangleObstacle& r, const Vec3& p, const Vec3& f) {
    Agent v;
    v.reset();
    v.setPosition(p);
    v.regenerateOrthonormalBasisUF(f.normalize());
    AbstractObstacle::PathIntersection pi;
    r.findIntersectionWithVehiclePath(v, pi);
    std::printf("hit %s  %.6f %.6f %.6f  %.6f %.6f %.6f  %d", name, p.x, p.y, p.z, f.x, f.y, f.z, pi.intersect ? 1 : 0);
    if (pi.intersect) {
        const Vec3 h = pi.steerHint, s = pi.surfacePoint;
        std::printf("  %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f", pi.distance, h.x, h.y, h.z, s.x, s.y, s.z);
    }
    std::printf("\n");
}

int main() {
    const Vec3 positions[] = {Vec3(0, 1, 0), Vec3(2.9f, 1, 0),   Vec3(3.6f, 1, 0), Vec3(0, 2.4f, 0),
                              Vec3(0, 2.6f, 0), Vec3(1, 1, 8), Vec3(0, 1, 4),    Vec3(1, 2, -6)};
    const Vec3 forwards[] = {Vec3(0, 0, 1),     Vec3(0, 0, -1), Vec3(1, 0, 1),
                             Vec3(-0.3f, 0.2f, 1), Vec3(1, 0, 0), Vec3(0.2f, -0.5f, 1)};
    const char* names[] = {"outside", "inside", "both", "tilted"};
    const RectangleObstacle rects[] = {wall(Obstacle::outside), wall(Obstacle::inside), wall(Obstacle::both), tilted()};
    for (int r = 0; r < 4; ++r)
        for (const Vec3& p : positions)
            for (const Vec3& f : forwards) hit(names[r], rects[r], p, f);

    RectangleObstacle upright = wall(Obstacle::both);
    RectangleObstacle slanted = tilted();
    ObstacleGroup obstacles;
    obstacles.push_back(&upright);
    obstacles.push_back(&slanted);
    const Vec3 targets[] = {Vec3(0.5f, 1, 10), Vec3(-0.5f, 0.5f, -8)};

    // At rest at the origin facing +z, maxForce 5, maxSpeed 3; the target changes every 5 s.
    Agent v;
    v.reset();
    v.setSpeed(0);
    v.setMaxForce(5);
    v.setMaxSpeed(3);
    const float dt = 1.0f / 60.0f;
    int avoiding = 0;
    for (int step = 1; step <= 1800; ++step) {
        const Vec3 target = targets[((step - 1) / 300) % 2];
        const Vec3 avoidance = v.steerToAvoidObstacles(1.0f, obstacles);
        Vec3 steer = avoidance;
        if (avoidance == Vec3::zero) {
            steer = v.xxxsteerForSeek(target);
        } else {
            ++avoiding;
        }
        v.applySteeringForce(steer, dt);
        if (step == 1 || step % 45 == 0) {
            const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
            std::printf("walls %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %d\n",
                        step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed(), avoiding);
        }
    }
}
