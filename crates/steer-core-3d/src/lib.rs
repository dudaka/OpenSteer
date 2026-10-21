//! 3D steering behaviors ported from OpenSteer as it is (`Vec3`, a forward/side/up basis), next
//! to the 2D port in `steer-core`. Pure computation, no IO.

pub mod util;
mod vec3;
mod vehicle;

pub use vec3::Vec3;
pub use vehicle::Vehicle;
