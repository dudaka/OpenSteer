//! 3D steering behaviors ported from OpenSteer as it is (`Vec3`, a forward/side/up basis), next
//! to the 2D port in `steer-core`. Pure computation, no IO.

pub mod containment;
pub mod flock;
pub mod leader;
pub mod neighbors;
pub mod obstacle;
pub mod pathway;
pub mod steer;
pub mod util;
mod vec3;
mod vehicle;
pub mod wall;

pub use vec3::Vec3;
pub use vehicle::Vehicle;
