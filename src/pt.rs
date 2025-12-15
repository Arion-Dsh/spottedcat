#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pt(pub f32);

impl Pt {
    pub fn as_f32(self) -> f32 {
        self.0
    }

    pub(crate) fn from_physical_px(px: f64, scale_factor: f64) -> Self {
        Pt((px / scale_factor) as f32)
    }

    pub(crate) fn to_physical_px(self, scale_factor: f64) -> f64 {
        (self.0 as f64) * scale_factor
    }
}

impl From<u32> for Pt {
    fn from(value: u32) -> Self {
        Pt(value as f32)
    }
}

impl From<u16> for Pt {
    fn from(value: u16) -> Self {
        Pt(value as f32)
    }
}

impl From<u8> for Pt {
    fn from(value: u8) -> Self {
        Pt(value as f32)
    }
}

impl From<usize> for Pt {
    fn from(value: usize) -> Self {
        Pt(value as f32)
    }
}

impl From<i32> for Pt {
    fn from(value: i32) -> Self {
        Pt(value as f32)
    }
}

impl From<i64> for Pt {
    fn from(value: i64) -> Self {
        Pt(value as f32)
    }
}

impl From<f32> for Pt {
    fn from(value: f32) -> Self {
        Pt(value)
    }
}

impl From<f64> for Pt {
    fn from(value: f64) -> Self {
        Pt(value as f32)
    }
}

impl std::ops::Add for Pt {
    type Output = Pt;
    fn add(self, rhs: Pt) -> Pt {
        Pt(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Pt {
    type Output = Pt;
    fn sub(self, rhs: Pt) -> Pt {
        Pt(self.0 - rhs.0)
    }
}

impl std::ops::AddAssign for Pt {
    fn add_assign(&mut self, rhs: Pt) {
        self.0 += rhs.0;
    }
}

impl std::ops::SubAssign for Pt {
    fn sub_assign(&mut self, rhs: Pt) {
        self.0 -= rhs.0;
    }
}

impl std::ops::Mul<f32> for Pt {
    type Output = Pt;
    fn mul(self, rhs: f32) -> Pt {
        Pt(self.0 * rhs)
    }
}

impl std::ops::Div<f32> for Pt {
    type Output = Pt;
    fn div(self, rhs: f32) -> Pt {
        Pt(self.0 / rhs)
    }
}

impl std::ops::MulAssign<f32> for Pt {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl std::ops::DivAssign<f32> for Pt {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}
