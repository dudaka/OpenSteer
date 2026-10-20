//! View models sent to the browser every frame. Points are `[x, y]` in 2D world units;
//! colors are `0xRRGGBB`.

use serde::Serialize;
use ts_rs::TS;

/// One vehicle as drawn: OpenSteer's basic 2D body, a triangle in a boundary circle.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct VehicleView {
    pub position: [f32; 2],
    /// Unit heading.
    pub forward: [f32; 2],
    pub radius: f32,
    pub color: u32,
}

/// An annotation line segment.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LineView {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: u32,
}

/// A vehicle's recent path, oldest point first.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TrailView {
    pub points: Vec<[f32; 2]>,
    pub color: u32,
}

/// Text drawn at a world position.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LabelView {
    pub at: [f32; 2],
    pub text: String,
    pub color: u32,
}

/// The whole world at one simulation step.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FrameView {
    /// Simulation time in seconds.
    pub time: f32,
    /// World point the camera centres on.
    pub focus: [f32; 2],
    pub vehicles: Vec<VehicleView>,
    pub lines: Vec<LineView>,
    pub trails: Vec<TrailView>,
    pub labels: Vec<LabelView>,
}
