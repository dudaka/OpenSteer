//! 3D view models sent to the browser every frame. Points are `[x, y, z]` in world units with
//! +y up, drawn by three.js as they are; colors are `0xRRGGBB`. Buttons, sliders and the legend
//! are the 2D demos' types.

use serde::Serialize;
use steer_demos::{ActionView, LegendView, ParamView};
use ts_rs::TS;

/// One vehicle: a flat triangle in its forward/side plane, so banking shows as a roll.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Vehicle3View {
    pub position: [f32; 3],
    /// Unit heading.
    pub forward: [f32; 3],
    /// Unit up, perpendicular to `forward`.
    pub up: [f32; 3],
    pub radius: f32,
    pub color: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Line3View {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub color: u32,
}

/// A vehicle's recent path, oldest point first.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Trail3View {
    pub points: Vec<[f32; 3]>,
    pub color: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Label3View {
    pub at: [f32; 3],
    pub text: String,
    pub color: u32,
}

/// The ground under a 3D demo, sent when the demo is chosen (not every frame): a square grid of
/// `size` x `size` heights `spacing` apart from `origin` (x, z), row by row along z.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TerrainView {
    pub origin: [f32; 2],
    pub spacing: f32,
    pub size: u32,
    pub heights: Vec<f32>,
    pub color: u32,
}

/// The whole 3D world at one simulation step.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Frame3View {
    /// Simulation time in seconds.
    pub time: f32,
    /// World point the camera orbits around and follows.
    pub focus: [f32; 3],
    /// Starting distance from the camera to the focus.
    pub view_size: f32,
    pub vehicles: Vec<Vehicle3View>,
    pub lines: Vec<Line3View>,
    pub trails: Vec<Trail3View>,
    pub labels: Vec<Label3View>,
    pub actions: Vec<ActionView>,
    pub params: Vec<ParamView>,
    /// What each line color means, for every color this frame draws.
    pub legend: Vec<LegendView>,
}
