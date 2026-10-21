// Oracle: OpenSteer's Soccer plug-in logic (plugins/Soccer.cpp, drawing removed), run headless.
// Deterministic: every player starts from playerPosition (the plug-in's frandom draws are
// overwritten). Each Player keeps the copy of m_AllPlayers made when it was constructed, so it
// avoids only the players created before it. Built without fused multiply-add, as Rust
// computes. Build and run from the repo root:
//   c++ -std=c++11 -w -ffp-contract=off -Iinclude -o /tmp/soccer oracle/soccer.cpp \
//     oracle/annotation_stubs.cpp src/SimpleVehicle.cpp src/Vec3.cpp src/Vec3Utilities.cpp \
//     src/Color.cpp && /tmp/soccer
// Its output is pinned in crates/steer-demos/tests/oracle_soccer.rs.
#include <cstdio>
#include <vector>
#include "OpenSteer/SimpleVehicle.h"

using namespace OpenSteer;

const Vec3 playerPosition[8] = {Vec3(4, 0, 0),   Vec3(7, 0, -5),  Vec3(7, 0, 5),
                                Vec3(10, 0, -3), Vec3(10, 0, 3),  Vec3(15, 0, -8),
                                Vec3(15, 0, 0),  Vec3(15, 0, 8)};

struct AABBox {
    Vec3 m_min, m_max;
    bool InsideX(const Vec3 p) { return !(p.x < m_min.x || p.x > m_max.x); }
    bool InsideZ(const Vec3 p) { return !(p.z < m_min.z || p.z > m_max.z); }
};

AABBox field{Vec3(-20, 0, -10), Vec3(20, 0, 10)};
AABBox goalA{Vec3(-21, 0, -7), Vec3(-19, 0, 7)};
AABBox goalB{Vec3(19, 0, -7), Vec3(21, 0, 7)};

struct Ball : SimpleVehicle {
    void update(const float, const float) {}
    void restart() {
        SimpleVehicle::reset();
        setSpeed(0.0f);
        setMaxForce(9.0f);
        setMaxSpeed(9.0f);
        setPosition(0, 0, 0);
    }
    void step(const float elapsedTime) {
        applyBrakingForce(1.5f, elapsedTime);
        applySteeringForce(velocity(), elapsedTime);
        if (!field.InsideX(position())) {
            Vec3 d = velocity();
            regenerateOrthonormalBasis(Vec3(-d.x, d.y, d.z));
            applySteeringForce(velocity(), elapsedTime);
        }
        if (!field.InsideZ(position())) {
            Vec3 d = velocity();
            regenerateOrthonormalBasis(Vec3(d.x, d.y, -d.z));
            applySteeringForce(velocity(), elapsedTime);
        }
    }
    void kick(Vec3 dir) {
        setSpeed(dir.length());
        regenerateOrthonormalBasis(dir);
    }
};

Ball ball;

struct Player : SimpleVehicle {
    void update(const float, const float) {}
    AVGroup others;  // the players constructed before this one
    bool teamA;
    Vec3 home;

    void place(bool isTeamA, int id) {
        teamA = isTeamA;
        SimpleVehicle::reset();
        setSpeed(0.0f);
        setMaxForce(3000.7f);
        setMaxSpeed(10);
        const Vec3 p = playerPosition[id];
        setPosition(teamA ? p : Vec3(-p.x, p.y, p.z));
        home = position();
    }

    void step(const float elapsedTime) {
        const float distToBall = Vec3::distance(position(), ball.position());
        if (distToBall < radius() + ball.radius()) ball.kick((ball.position() - position()) * 50);
        const Vec3 collisionAvoidance = steerToAvoidNeighbors(1, others);
        if (collisionAvoidance != Vec3::zero) {
            applySteeringForce(collisionAvoidance, elapsedTime);
            return;
        }
        const float distHomeToBall = Vec3::distance(home, ball.position());
        if (distHomeToBall < 12.0f) {
            if (teamA ? position().x > ball.position().x : position().x < ball.position().x) {
                applySteeringForce(xxxsteerForSeek(ball.position()), elapsedTime);
            } else {
                const float Z = ball.position().z - position().z > 0 ? -1.0f : 1.0f;
                const Vec3 behindBall =
                    ball.position() + (teamA ? Vec3(2.0f, 0.0f, Z) : Vec3(-2.0f, 0.0f, Z));
                const Vec3 behindBallForce = xxxsteerForSeek(behindBall);
                const Vec3 evadeTarget = xxxsteerForFlee(ball.position());
                applySteeringForce(behindBallForce * 10.0f + evadeTarget, elapsedTime);
            }
        } else {
            applySteeringForce(xxxsteerForSeek(home) + xxxsteerForSeek(home), elapsedTime);
        }
    }
};

int main() {
    std::vector<Player> players(16);
    AVGroup all;
    for (int i = 0; i < 16; ++i) {
        players[i].others = all;
        players[i].place(i < 8, i % 8);
        all.push_back(&players[i]);
    }
    ball.restart();
    int red = 0, blue = 0;
    const float dt = 1.0f / 60.0f;
    for (int step = 1; step <= 3600; ++step) {
        for (Player& p : players) p.step(dt);
        ball.step(dt);
        if (goalA.InsideX(ball.position()) && goalA.InsideZ(ball.position())) {
            ball.restart();
            ++red;
        }
        if (goalB.InsideX(ball.position()) && goalB.InsideZ(ball.position())) {
            ball.restart();
            ++blue;
        }
        if (step % 120 == 0)
            std::printf("soccer %d %.6f %.6f %.6f %d %d %.6f %.6f %.6f %.6f\n", step,
                        ball.position().x, ball.position().z, ball.speed(), red, blue,
                        players[0].position().x, players[0].position().z,
                        players[15].position().x, players[15].position().z);
    }
}
