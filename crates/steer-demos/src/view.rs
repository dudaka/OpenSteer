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

/// A demo-specific button (OpenSteerDemo's per-plug-in function keys); clicking it sends
/// `demo.action` with `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ActionView {
    pub id: String,
    pub label: String,
    /// Shown pressed, e.g. the current mode.
    pub active: bool,
}

/// A demo-specific slider; moving it sends `demo.param` with `id` and the new value.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ParamView {
    pub id: String,
    pub label: String,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
}

/// One entry of the scene's legend: what a line color means in this demo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LegendView {
    pub color: u32,
    pub label: String,
}

/// The whole world at one simulation step.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FrameView {
    /// Simulation time in seconds.
    pub time: f32,
    /// World point the camera centres on.
    pub focus: [f32; 2],
    /// World units visible across the shorter side of the view.
    pub view_size: f32,
    pub vehicles: Vec<VehicleView>,
    pub lines: Vec<LineView>,
    pub trails: Vec<TrailView>,
    pub labels: Vec<LabelView>,
    pub actions: Vec<ActionView>,
    pub params: Vec<ParamView>,
    /// What each line color means, for every color this frame draws.
    pub legend: Vec<LegendView>,
}
