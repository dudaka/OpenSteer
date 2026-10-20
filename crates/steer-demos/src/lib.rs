//! One module per demo (the OpenSteer plug-ins): world setup, per-step update and the view
//! model the daemon sends to the browser.

mod circling;
mod view;

use serde::Serialize;
use ts_rs::TS;

pub use circling::Circling;
pub use view::{FrameView, VehicleView};

/// A demo the web app can select, in sidebar order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DemoInfo {
    /// Stable id used in commands, e.g. `one-turning`.
    pub id: String,
    /// Name shown in the sidebar.
    pub name: String,
}

/// Every demo, in sidebar order. Empty until the first demo is ported.
pub fn catalog() -> Vec<DemoInfo> {
    Vec::new()
}
