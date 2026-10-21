//! 2D steering behaviors ported from OpenSteer: vector math, the vehicle and its integrator,
//! and the steering forces. Pure computation, no IO.

pub mod containment;
pub mod flock;
pub mod leader;
pub mod neighbors;
pub mod obstacle;
pub mod pathway;
pub mod proximity;
pub mod steer;
pub mod util;
mod vec2;
mod vehicle;
pub mod wall;

pub use vec2::Vec2;
pub use vehicle::Vehicle;
