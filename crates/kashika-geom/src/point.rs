use std::ops::{Add, Mul, Sub};

use serde::Serialize;

pub const TOLERANCE: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn dist(self, other: Pt) -> f64 {
        (self - other).length()
    }

    pub fn norm(self) -> Pt {
        let l = self.length();
        if l == 0.0 {
            self
        } else {
            Pt::new(self.x / l, self.y / l)
        }
    }

    pub fn lerp(self, other: Pt, t: f64) -> Pt {
        self + (other - self) * t
    }
}

impl Add for Pt {
    type Output = Pt;
    fn add(self, o: Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Pt {
    type Output = Pt;
    fn sub(self, o: Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Pt {
    type Output = Pt;
    fn mul(self, k: f64) -> Pt {
        Pt::new(self.x * k, self.y * k)
    }
}
