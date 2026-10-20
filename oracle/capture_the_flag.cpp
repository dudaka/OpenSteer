// Oracle: OpenSteer's CaptureTheFlag plug-in logic (plugins/CaptureTheFlag.cpp, drawing and
// random placement removed), run headless from a fixed layout: 8 rocks, a seeker and 4 enemies.
// The seeker runs for home (seek, XXXsteerToEvadeAllDefenders, obstacle avoidance with a
// look-ahead switched by clearPathToGoal); enemies pursue it; after a tag or reaching home all
// brake. Built without fused multiply-add, as Rust computes. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/capture_the_flag \
//     oracle/capture_the_flag.cpp oracle/annotation_stubs.cpp src/SimpleVehicle.cpp \
//     src/Obstacle.cpp src/Vec3.cpp src/Vec3Utilities.cpp src/Color.cpp && /tmp/capture_the_flag
// Its output is pinned in crates/steer-demos/tests/oracle_capture_the_flag.rs.
#include <cstdio>
#include "OpenSteer/SimpleVehicle.h"
#include "OpenSteer/Obstacle.h"

using namespace OpenSteer;

const Vec3 gHomeBaseCenter(0, 0, 0);
const float gHomeBaseRadius = 1.5;
const float gBrakingRate = 0.75;
const float gAvoidancePredictTimeMin = 0.9f;
const float gAvoidancePredictTimeMax = 2;
float gAvoidancePredictTime = gAvoidancePredictTimeMin;
ObstacleGroup allObstacles;
enum seekerState { running, tagged, atGoal };

struct CtfBase : SimpleVehicle {
    void update(const float, const float) {}
    // CtfBase::reset without the random position and heading.
    void place(const Vec3& at, const Vec3& heading) {
        reset();
        setSpeed(3);
        setMaxForce(3.0);
        setMaxSpeed(3.0);
        setPosition(at);
        regenerateOrthonormalBasisUF(heading.normalize());
    }
};

const int ctfEnemyCount = 4;
CtfBase enemies[ctfEnemyCount];

struct CtfSeeker : CtfBase {
    seekerState state = running;

    bool clearPathToGoal() {
        const float sideThreshold = radius() * 8.0f;
        const float behindThreshold = radius() * 2.0f;
        const Vec3 goalOffset = gHomeBaseCenter - position();
        const float goalDistance = goalOffset.length();
        const Vec3 goalDirection = goalOffset / goalDistance;
        const bool goalIsAside = isAside(gHomeBaseCenter, 0.5);
        bool xxxReturn = true;
        for (int i = 0; i < ctfEnemyCount; i++) {
            const CtfBase& e = enemies[i];
            const float eDistance = Vec3::distance(position(), e.position());
            const float timeEstimate = 0.3f * eDistance / e.speed();
            const Vec3 eFuture = e.predictFuturePosition(timeEstimate);
            const Vec3 eOffset = eFuture - position();
            const float alongCorridor = goalDirection.dot(eOffset);
            const bool inCorridor =
                ((alongCorridor > -behindThreshold) && (alongCorridor < goalDistance));
            const float eForwardDistance = forward().dot(eOffset);
            if (inCorridor) {
                const Vec3 perp = eOffset - (goalDirection * alongCorridor);
                const float acrossCorridor = perp.length();
                if (acrossCorridor < sideThreshold) {
                    const float eFront = eForwardDistance + e.radius();
                    const bool eIsBehind = eFront < -behindThreshold;
                    const bool eIsWayBehind = eFront < (-2 * behindThreshold);
                    const bool safeToTurnTowardsGoal =
                        ((eIsBehind && goalIsAside) || eIsWayBehind);
                    if (!safeToTurnTowardsGoal) xxxReturn = false;
                }
            }
        }
        return xxxReturn;
    }

    Vec3 XXXsteerToEvadeAllDefenders() {
        Vec3 evade(0, 0, 0);
        for (int i = 0; i < ctfEnemyCount; i++) {
            const CtfBase& e = enemies[i];
            const Vec3 eOffset = e.position() - position();
            const float eDistance = eOffset.length();
            const float timeEstimate = 0.5f * eDistance / e.speed();
            const Vec3 eFuture = e.predictFuturePosition(timeEstimate);
            const Vec3 flee = xxxsteerForFlee(eFuture);
            const float eForwardDistance = forward().dot(eOffset);
            const float behindThreshold = radius() * -2;
            const float distanceWeight = 4 / eDistance;
            const float forwardWeight = ((eForwardDistance > behindThreshold) ? 1.0f : 0.5f);
            evade += flee * distanceWeight * forwardWeight;
        }
        return evade;
    }

    void adjustObstacleAvoidanceLookAhead(const bool clearPath) {
        if (clearPath) {
            const float goalDistance = Vec3::distance(gHomeBaseCenter, position());
            const bool headingTowardGoal = isAhead(gHomeBaseCenter, 0.98f);
            const bool isNear = (goalDistance / speed()) < gAvoidancePredictTimeMax;
            const bool useMax = headingTowardGoal && !isNear;
            gAvoidancePredictTime = (useMax ? gAvoidancePredictTimeMax : gAvoidancePredictTimeMin);
        } else {
            gAvoidancePredictTime = gAvoidancePredictTimeMin;
        }
    }

    Vec3 steeringForSeeker() {
        const bool clearPath = clearPathToGoal();
        adjustObstacleAvoidanceLookAhead(clearPath);
        const Vec3 obstacleAvoidance = steerToAvoidObstacles(gAvoidancePredictTime, allObstacles);
        if (obstacleAvoidance != Vec3::zero) return obstacleAvoidance;
        const Vec3 seek = xxxsteerForSeek(gHomeBaseCenter);
        if (clearPath) return limitMaxDeviationAngle(seek, 0.707f, forward());
        const Vec3 evade = XXXsteerToEvadeAllDefenders();
        return limitMaxDeviationAngle(seek + evade, 0.707f, forward());
    }

    // CtfSeeker::update without updateState's reset timer.
    void step(const float elapsedTime) {
        if (state == running &&
            Vec3::distance(position(), gHomeBaseCenter) < (radius() + gHomeBaseRadius))
            state = atGoal;
        Vec3 steer(0, 0, 0);
        if (state == running)
            steer = steeringForSeeker();
        else
            applyBrakingForce(gBrakingRate, elapsedTime);
        applySteeringForce(steer, elapsedTime);
    }
};

CtfSeeker seeker;

// CtfEnemy::update.
void enemyStep(CtfBase& e, const float elapsedTime) {
    const float seekerToGoalDist = Vec3::distance(gHomeBaseCenter, seeker.position());
    const float adjustedDistance = seekerToGoalDist - e.radius() - gHomeBaseRadius;
    const float seekerToGoalTime =
        ((adjustedDistance < 0) ? 0 : (adjustedDistance / seeker.speed()));
    const float maxPredictionTime = seekerToGoalTime * 0.9f;
    Vec3 steer(0, 0, 0);
    if (seeker.state == running) {
        const Vec3 avoidance = e.steerToAvoidObstacles(gAvoidancePredictTimeMin, allObstacles);
        if (avoidance == Vec3::zero)
            steer = e.steerForPursuit(seeker, maxPredictionTime);
        else
            steer = avoidance;
    } else {
        e.applyBrakingForce(gBrakingRate, elapsedTime);
    }
    e.applySteeringForce(steer, elapsedTime);
    if (Vec3::distance(e.position(), seeker.position()) < e.radius() + seeker.radius())
        if (seeker.state == running) seeker.state = tagged;
}

void run(const char* name, const Vec3& seekerAt, const Vec3& seekerHeading) {
    const float rocks[8][3] = {{10, 5, 2},   {-8, 12, 3},   {15, -10, 2.5f}, {-20, -5, 3.5f},
                               {25, 15, 2},  {0, -18, 1.5f}, {-30, 20, 4},   {30, -25, 3}};
    allObstacles.clear();
    for (const auto& r : rocks) allObstacles.push_back(new SphereObstacle(r[2], Vec3(r[0], 0, r[1])));
    gAvoidancePredictTime = gAvoidancePredictTimeMin;
    seeker.place(seekerAt, seekerHeading);
    seeker.state = running;
    const float e[4][4] = {{20, 30, 1, -1}, {-25, -30, 1, 1}, {5, -35, -1, 1}, {-38, 5, 1, 0}};
    for (int i = 0; i < ctfEnemyCount; i++)
        enemies[i].place(Vec3(e[i][0], 0, e[i][1]), Vec3(e[i][2], 0, e[i][3]));
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 1200; ++step) {
        seeker.step(dt);
        for (int i = 0; i < ctfEnemyCount; i++) enemyStep(enemies[i], dt);
        if (step % 60 == 0) {
            std::printf("%s %d %d %.6f %.6f %.6f %.6f %.6f", name, step, (int)seeker.state,
                        seeker.position().x, seeker.position().z, seeker.forward().x,
                        seeker.forward().z, seeker.speed());
            for (int i = 0; i < ctfEnemyCount; i++)
                std::printf(" %.6f %.6f", enemies[i].position().x, enemies[i].position().z);
            std::printf("\n");
        }
    }
}

int main() {
    run("north", Vec3(36, 0, 8), Vec3(-0.6f, 0, 0.8f));
    run("south", Vec3(-12, 0, -36), Vec3(0, 0, 1));
}
