//! Builders for a demo's buttons and sliders.

use crate::view::{ActionView, ParamView};

/// A slider's range and step.
#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl Range {
    pub const fn new(min: f32, max: f32, step: f32) -> Self {
        Self { min, max, step }
    }

    /// `value` held inside the range.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

pub fn button(id: &str, label: &str, active: bool) -> ActionView {
    ActionView {
        id: id.to_string(),
        label: label.to_string(),
        active,
    }
}

pub fn slider(id: &str, label: &str, range: Range, value: f32) -> ParamView {
    ParamView {
        id: id.to_string(),
        label: label.to_string(),
        min: range.min,
        max: range.max,
        step: range.step,
        value,
    }
}
