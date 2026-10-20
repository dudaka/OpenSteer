//! Placeholder world: one dot moving on a circle, to prove the frame stream. Replaced by the
//! first ported demo.

use crate::view::{FrameView, VehicleView};

const ORBIT_RADIUS: f32 = 10.0;
const ANGULAR_SPEED: f32 = 1.0;

#[derive(Debug, Default)]
pub struct Circling {
    time: f32,
}

impl Circling {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
    }

    pub fn view(&self) -> FrameView {
        let angle = self.time * ANGULAR_SPEED;
        FrameView {
            time: self.time,
            vehicles: vec![VehicleView {
                x: ORBIT_RADIUS * angle.cos(),
                y: ORBIT_RADIUS * angle.sin(),
                radius: 0.5,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_stays_on_orbit_and_moves() {
        let mut world = Circling::default();
        let start = world.view().vehicles[0].clone();
        world.update(0.5);
        let v = world.view().vehicles[0].clone();
        assert!((v.x.hypot(v.y) - ORBIT_RADIUS).abs() < 1e-4);
        assert_ne!((v.x, v.y), (start.x, start.y));
        assert_eq!(world.view().time, 0.5);
    }
}
