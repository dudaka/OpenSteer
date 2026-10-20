//! One module per demo (the OpenSteer plug-ins): world setup, per-step update and the view
//! model the daemon sends to the browser.

mod annotation;
mod one_turning;
mod view;

use serde::Serialize;
use ts_rs::TS;

pub use one_turning::OneTurning;
pub use view::{FrameView, LabelView, LineView, TrailView, VehicleView};

/// A demo the web app can select, in sidebar order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DemoInfo {
    /// Stable id used in commands, e.g. `one-turning`.
    pub id: String,
    /// Name shown in the sidebar.
    pub name: String,
}

/// Every demo, in sidebar order (OpenSteerDemo's `selectionOrderSortKey`).
pub fn catalog() -> Vec<DemoInfo> {
    vec![DemoInfo {
        id: "one-turning".to_string(),
        name: "One Turning Away".to_string(),
    }]
}
