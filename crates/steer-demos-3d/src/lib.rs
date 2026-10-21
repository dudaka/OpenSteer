//! The 3D demos: world setup, per-step update and the 3D view model the daemon sends to the
//! browser, next to the 2D ones in `steer-demos`.

mod annotation;
mod boids;
mod containment;
mod crossing;
mod demo;
mod ground;
mod leader_following;
mod obstacles;
mod one_turning;
mod path_following;
mod pursuit;
mod seek_flee;
mod silkroad_seek;
mod view;
mod wall_following;
mod wander;
pub mod world;

pub use boids::Boids3;
pub use containment::Containment3;
pub use crossing::Crossing3;
pub use demo::{catalog, Demo3};
pub use leader_following::LeaderFollowing3;
pub use obstacles::Obstacles3;
pub use one_turning::OneTurning3;
pub use path_following::PathFollowing3;
pub use pursuit::Pursuit3;
pub use seek_flee::SeekFlee3;
pub use silkroad_seek::SilkroadSeek3;
pub use view::{Frame3View, Label3View, Line3View, Trail3View, Vehicle3View};
pub use wall_following::WallFollowing3;
pub use wander::Wander3;
