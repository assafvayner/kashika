use serde::Serialize;

use crate::Pt;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn origin(self) -> Pt {
        Pt::new(self.x, self.y)
    }

    pub fn center(self) -> Pt {
        Pt::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }
}
