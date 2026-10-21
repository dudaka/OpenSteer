//! One module per demo (the OpenSteer plug-ins): world setup, per-step update and the view
//! model the daemon sends to the browser.

mod annotation;
mod boids;
mod capture_the_flag;
mod containment;
mod controls;
mod crossing;
mod demo;
mod leader_following;
mod obstacles;
mod one_turning;
mod path_following;
mod pedestrians;
mod pursuit;
mod seek_flee;
mod soccer;
mod view;
mod wall_following;
mod wander;

use serde::Serialize;
use ts_rs::TS;

pub use boids::Boids;
pub use capture_the_flag::{CaptureTheFlag, SeekerState};
pub use containment::Containment;
pub use crossing::Crossing;
pub use demo::{catalog, Demo, DEFAULT_DEMO};
pub use leader_following::LeaderFollowing;
pub use obstacles::Obstacles;
pub use one_turning::OneTurning;
pub use path_following::PathFollowing;
pub use pedestrians::Pedestrians;
pub use pursuit::Pursuit;
pub use seek_flee::SeekFlee;
pub use soccer::Soccer;
pub use view::{ActionView, FrameView, LabelView, LineView, ParamView, TrailView, VehicleView};
pub use wall_following::WallFollowing;
pub use wander::Wander;

/// A demo the web app can select, in sidebar order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DemoInfo {
    /// Stable id used in commands, e.g. `one-turning`.
    pub id: String,
    /// Name shown in the sidebar.
    pub name: String,
}
