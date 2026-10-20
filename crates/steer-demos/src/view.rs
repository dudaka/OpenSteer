//! View models sent to the browser every frame. Positions are 2D world units.

use serde::Serialize;
use ts_rs::TS;

/// One vehicle as drawn: a disc at `(x, y)`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VehicleView {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

/// The whole world at one simulation step.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FrameView {
    /// Simulation time in seconds.
    pub time: f32,
    pub vehicles: Vec<VehicleView>,
}
