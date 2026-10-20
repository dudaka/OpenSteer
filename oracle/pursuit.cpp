// Oracle: OpenSteer's steerForPursuit (MultiplePursuit: a pursuer chasing a wanderer) and
// steerForEvasion, run headless. steerForEvasion and the plain steerForFlee it calls do not
// compile (`this->menace.position`, `position - target`), so `evasion` below is their code with
// `position()` written as intended. rand() is the LCG of oracle/wander.cpp. Build and run from the
// repo root:
//   c++ -std=c++11 -w -Drand=oracle_rand -Iinclude -o /tmp/pursuit oracle/pursuit.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp \
//     src/Color.cpp && /tmp/pursuit
// Its output is pinned in crates/steer-core/tests/oracle_pursuit.rs.
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

void print(const char* name, int step, const Agent& p, const Agent& q) {
    std::printf("%s %d %.6f %.6f %.6f %.6f %.6f %.6f %.6f\n", name, step, p.position().x,
                p.position().z, p.forward().x, p.forward().z, p.speed(), q.position().x,
                q.position().z);
}

int main() {
    const float dt = 1.0f / 60.0f;

    // A pursuer from (8, -6) chases MpWanderer (forward + 3 * wander), maxTime 20. The wanderer
    // moves first, as in MpPlugIn::update.
    lcg = 1;
    Agent wanderer, pursuer;
    setUp(wanderer, Vec3(0, 0, 0));
    setUp(pursuer, Vec3(8, 0, -6));
    for (int step = 1; step <= 600; ++step) {
        const Vec3 wander2d = wanderer.steerForWander(dt).setYtoZero();
        wanderer.applySteeringForce(wanderer.forward() + wander2d * 3, dt);
        pursuer.applySteeringForce(pursuer.steerForPursuit(wanderer, 20), dt);
        if (step % 60 == 0) print("pursuit", step, pursuer, wanderer);
    }

    // An evader (maxSpeed 2) evades, maxTime 2, a pursuer that pursues it, maxTime 20.
    Agent evader, hunter;
    setUp(evader, Vec3(0, 0, 0));
    setUp(hunter, Vec3(-5, 0, -5));
    evader.setMaxSpeed(2);
    for (int step = 1; step <= 600; ++step) {
        evader.applySteeringForce(evasion(evader, hunter, 2), dt);
        hunter.applySteeringForce(hunter.steerForPursuit(evader, 20), dt);
        if (step % 60 == 0) print("evasion", step, evader, hunter);
    }
}
