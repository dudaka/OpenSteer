// Oracle: OpenSteer's steerForPursuit (a pursuer chasing a wanderer) and steerForEvasion in 3D
// (no setYtoZero), run headless. steerForEvasion and the plain steerForFlee it calls do not
// compile (`this->menace.position`, `position - target`), so `evasion` below is their code with
// `position()` written as intended. rand() is the LCG of oracle/wander_3d.cpp. Build and run from
// the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Drand=oracle_rand -Iinclude -o /tmp/pursuit_3d \
//     oracle/pursuit_3d.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp \
//     src/Vec3Utilities.cpp src/Color.cpp && /tmp/pursuit_3d
// Its output is pinned in crates/steer-core-3d/tests/oracle_pursuit_3d.rs.
#include <cstdio>
#include <cstdlib>
#include "OpenSteer/SimpleVehicle.h"

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

// steerForEvasion, then plain steerForFlee, with position() as intended.
Vec3 evasion(Agent& self, const AbstractVehicle& menace, const float maxPredictionTime) {
    const Vec3 offset = menace.position() - self.position();
    const float distance = offset.length();
    const float roughTime = distance / menace.speed();
    const float predictionTime = (roughTime > maxPredictionTime) ? maxPredictionTime : roughTime;
    const Vec3 target = menace.predictFuturePosition(predictionTime);
    const Vec3 desiredVelocity = self.position() - target;
    return desiredVelocity - self.velocity();
}

// MpBase::reset: at rest facing +z, maxForce 5, maxSpeed 3.
void setUp(Agent& v, const Vec3& at) {
    v.reset();
    v.setSpeed(0);
    v.setMaxForce(5);
    v.setMaxSpeed(3);
    v.setPosition(at);
}

void print(const char* name, int step, const Agent& v) {
    const Vec3 p = v.position(), f = v.forward(), s = v.side(), u = v.up();
    std::printf("%s %d %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f %.6f %.6f  %.6f\n", name,
                step, p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed());
}

int main() {
    const float dt = 1.0f / 60.0f;

    // A pursuer from (8, 3, -6) chases a 3D wanderer (forward + 3 * wander), maxTime 20. The
    // wanderer moves first, as in MpPlugIn::update. Prints step 1 and every 60.
    lcg = 1;
    Agent wanderer, pursuer;
    setUp(wanderer, Vec3(0, 0, 0));
    setUp(pursuer, Vec3(8, 3, -6));
    for (int step = 1; step <= 600; ++step) {
        wanderer.applySteeringForce(wanderer.forward() + wanderer.steerForWander(dt) * 3, dt);
        pursuer.applySteeringForce(pursuer.steerForPursuit(wanderer, 20), dt);
        if (step == 1 || step % 60 == 0) {
            print("pursuer", step, pursuer);
            print("wanderer", step, wanderer);
        }
    }

    // An evader (maxSpeed 2) evades, maxTime 2, a hunter from (-5, -2, -5) that pursues it,
    // maxTime 20.
    Agent evader, hunter;
    setUp(evader, Vec3(0, 0, 0));
    setUp(hunter, Vec3(-5, -2, -5));
    evader.setMaxSpeed(2);
    for (int step = 1; step <= 600; ++step) {
        evader.applySteeringForce(evasion(evader, hunter, 2), dt);
        hunter.applySteeringForce(hunter.steerForPursuit(evader, 20), dt);
        if (step == 1 || step % 60 == 0) {
            print("evader", step, evader);
            print("hunter", step, hunter);
        }
    }

    // Every cell of steerForPursuit's table, one call each: a pursuer at the origin facing +z at
    // speed 1; a quarry at speed 2, ahead / aside / behind and heading parallel / perpendicular /
    // anti-parallel (off the ground by 1). Prints the force.
    const Vec3 offsets[3] = {Vec3(0, 1, 5), Vec3(5, 1, 0), Vec3(0, 1, -5)};
    const Vec3 headings[3] = {Vec3(0, 0, 1), Vec3(1, 0, 0), Vec3(0, 0, -1)};
    for (int i = 0; i < 3; ++i) {
        for (int j = 0; j < 3; ++j) {
            Agent p, q;
            setUp(p, Vec3(0, 0, 0));
            p.setSpeed(1);
            setUp(q, offsets[i]);
            q.regenerateOrthonormalBasisUF(headings[j]);
            q.setSpeed(2);
            const Vec3 f = p.steerForPursuit(q, 20);
            std::printf("cell %d %d %.6f %.6f %.6f\n", i, j, f.x, f.y, f.z);
        }
    }
}
