//! Containment in 3D (Reynolds 1999, not in OpenSteer): keep a vehicle inside a box room. As the
//! 2D version: a probe ahead, and a full `max_force` push sideways toward the probe with the most
//! room, here among four probes 45 degrees toward +side, -side, +up and -up.

use crate::wall::Block;
use crate::{Vec3, Vehicle};

/// Distance from `p` to the room's walls, negative outside.
pub fn room(region: &Block, p: Vec3) -> f32 {
    -region.signed_distance(p)
}

/// The probes `radius + min_time * speed` from the center: ahead, then 45 degrees toward
/// +side, -side, +up and -up.
pub fn probes(v: &Vehicle, min_time: f32) -> [Vec3; 5] {
    let reach = v.radius + min_time * v.speed;
    let f = v.forward;
    [
        f,
        (f + v.side).normalize(),
        (f - v.side).normalize(),
        (f + v.up).normalize(),
        (f - v.up).normalize(),
    ]
    .map(|d| v.position + d * reach)
}

/// While the ahead probe is outside `region`: a `max_force` push along +side, -side, +up or
/// -up, toward the probe with the most room (the earlier one on a tie). Zero while the ahead
/// probe is inside. Only sideways, as in 2D: a braking push slows the vehicle into the
/// low-speed rule, which bends it forward again.
pub fn contain(v: &Vehicle, region: &Block, min_time: f32) -> Vec3 {
    let [ahead, probes @ ..] = probes(v, min_time);
    if region.contains(ahead) {
        return Vec3::ZERO;
    }
    let directions = [v.side, -v.side, v.up, -v.up];
    let (_, best) = probes
        .iter()
        .zip(directions)
        .map(|(&p, d)| (room(region, p), d))
        .reduce(|best, c| if c.0 > best.0 { c } else { best })
        .expect("four probes");
    best * v.max_force
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 x 10 x 10 room from the origin.
    fn room_box() -> Block {
        Block::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0))
    }

    /// Facing +z (side -x, up +y) at speed 2, max force 3.
    fn at(x: f32, y: f32, z: f32) -> Vehicle {
        Vehicle {
            position: Vec3::new(x, y, z),
            speed: 2.0,
            max_force: 3.0,
            ..Vehicle::default()
        }
    }

    #[test]
    fn no_push_while_the_probe_is_inside() {
        assert_eq!(contain(&at(5.0, 5.0, 5.0), &room_box(), 1.0), Vec3::ZERO);
    }

    /// Into a far bottom corner: every probe leaves through the far wall (z = 10) by 0.77; the
    /// +side probe also leaves through the -x wall and the -up probe through the floor, so they
    /// lose. -side and +up tie and the earlier, -side, wins: push +x.
    #[test]
    fn in_a_corner_probes_crossing_a_second_wall_lose() {
        let push = contain(&at(1.0, 1.0, 9.0), &room_box(), 1.0);
        assert_eq!(push, Vec3::new(3.0, 0.0, 0.0));
    }

    /// Near the far wall and the +x wall (at x = 9, mid height): +side is -x, toward the room.
    #[test]
    fn near_a_side_wall_turns_away_from_it() {
        let push = contain(&at(9.0, 5.0, 9.0), &room_box(), 1.0);
        assert_eq!(push, Vec3::new(-3.0, 0.0, 0.0));
    }
}
