//! The 3D demos: world setup, per-step update and the 3D view model the daemon sends to the
//! browser, next to the 2D ones in `steer-demos`.

mod annotation;
mod demo;
mod one_turning;
mod seek_flee;
mod view;
mod wander;

pub use demo::{catalog, Demo3};
pub use one_turning::OneTurning3;
pub use seek_flee::SeekFlee3;
pub use view::{Frame3View, Label3View, Line3View, Trail3View, Vehicle3View};
pub use wander::Wander3;
