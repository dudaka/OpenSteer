//! The ground-bound composite demos in the 3D view. OpenSteer's Capture the Flag, Pedestrians
//! and Soccer plug-ins flatten every force onto the ground (`setYtoZero`), so their 3D runs equal
//! the 2D ports (already checked against the C++); this lifts a 2D demo's frame onto the ground
//! plane: `(x, y)` is drawn at `(x, 0, y)`, with up straight up.

use steer_demos::{Demo, FrameView, LabelView, LineView, TrailView, VehicleView};

use crate::view::{Frame3View, Label3View, Line3View, Trail3View, Vehicle3View};

/// Annotations sit this far above the ground grid, so they do not flicker against it.
const LIFT: f32 = 0.02;

fn lift([x, y]: [f32; 2]) -> [f32; 3] {
    [x, LIFT, y]
}

/// A 2D demo shown on the ground of the 3D view.
#[derive(Debug, Clone)]
pub struct Ground(Box<Demo>);

impl Ground {
    /// The 2D demo `id`; `None` for an unknown id.
    pub fn new(id: &str) -> Option<Self> {
        Demo::new(id).map(|d| Ground(Box::new(d)))
    }

    pub fn update(&mut self, dt: f32) {
        self.0.update(dt);
    }

    pub fn action(&mut self, id: &str) {
        self.0.action(id);
    }

    pub fn param(&mut self, id: &str, value: f32) {
        self.0.param(id, value);
    }

    pub fn view(&self) -> Frame3View {
        lift_frame(self.0.view())
    }
}

fn lift_frame(f: FrameView) -> Frame3View {
    Frame3View {
        time: f.time,
        focus: [f.focus[0], 0.0, f.focus[1]],
        // A perspective camera this far away shows about as much as the 2D view's width.
        view_size: f.view_size * 1.1,
        vehicles: f.vehicles.into_iter().map(lift_vehicle).collect(),
        lines: f.lines.into_iter().map(lift_line).collect(),
        trails: f.trails.into_iter().map(lift_trail).collect(),
        labels: f.labels.into_iter().map(lift_label).collect(),
        actions: f.actions,
        params: f.params,
        legend: f.legend,
    }
}

fn lift_vehicle(v: VehicleView) -> Vehicle3View {
    Vehicle3View {
        position: [v.position[0], 0.0, v.position[1]],
        forward: [v.forward[0], 0.0, v.forward[1]],
        up: [0.0, 1.0, 0.0],
        radius: v.radius,
        color: v.color,
    }
}

fn lift_line(l: LineView) -> Line3View {
    Line3View {
        from: lift(l.from),
        to: lift(l.to),
        color: l.color,
    }
}

fn lift_trail(t: TrailView) -> Trail3View {
    Trail3View {
        points: t.points.into_iter().map(lift).collect(),
        color: t.color,
    }
}

fn lift_label(l: LabelView) -> Label3View {
    Label3View {
        at: lift(l.at),
        text: l.text,
        color: l.color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 3D view shows exactly the 2D run, lifted: same vehicles, positions at (x, 0, y).
    #[test]
    fn the_ground_view_is_the_2d_run_lifted() {
        let mut ground = Ground::new("soccer").unwrap();
        let mut flat = Demo::new("soccer").unwrap();
        for _ in 0..300 {
            ground.update(1.0 / 60.0);
            flat.update(1.0 / 60.0);
        }
        let (g, f) = (ground.view(), flat.view());
        assert_eq!(g.vehicles.len(), f.vehicles.len());
        for (a, b) in g.vehicles.iter().zip(&f.vehicles) {
            assert_eq!(a.position, [b.position[0], 0.0, b.position[1]]);
        }
        assert_eq!(g.legend, f.legend);
    }
}
