#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pt(pub u32);

impl Pt {
    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }

    pub(crate) fn from_physical_px(px: f64, scale_factor: f64) -> Self {
        let v = px / scale_factor;
        let v = if v.is_finite() { v } else { 0.0 };
        let v = v.round();
        let v = if v < 0.0 { 0.0 } else { v };
        Pt(v as u32)
    }

    pub(crate) fn to_physical_px(self, scale_factor: f64) -> f64 {
        (self.0 as f64) * scale_factor
    }
}

impl From<u32> for Pt {
    fn from(value: u32) -> Self {
        Pt(value)
    }
}

impl From<u16> for Pt {
    fn from(value: u16) -> Self {
        Pt(value as u32)
    }
}

impl From<u8> for Pt {
    fn from(value: u8) -> Self {
        Pt(value as u32)
    }
}

impl From<usize> for Pt {
    fn from(value: usize) -> Self {
        Pt(value as u32)
    }
}

impl From<i32> for Pt {
    fn from(value: i32) -> Self {
        Pt(value.max(0) as u32)
    }
}

impl From<i64> for Pt {
    fn from(value: i64) -> Self {
        Pt(value.max(0) as u32)
    }
}

impl From<f32> for Pt {
    fn from(value: f32) -> Self {
        let v = if value.is_finite() { value } else { 0.0 };
        let v = v.round();
        let v = if v < 0.0 { 0.0 } else { v };
        Pt(v as u32)
    }
}

impl From<f64> for Pt {
    fn from(value: f64) -> Self {
        let v = if value.is_finite() { value } else { 0.0 };
        let v = v.round();
        let v = if v < 0.0 { 0.0 } else { v };
        Pt(v as u32)
    }
}

impl std::ops::Add for Pt {
    type Output = Pt;
    fn add(self, rhs: Pt) -> Pt {
        Pt(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::Sub for Pt {
    type Output = Pt;
    fn sub(self, rhs: Pt) -> Pt {
        Pt(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::AddAssign for Pt {
    fn add_assign(&mut self, rhs: Pt) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl std::ops::SubAssign for Pt {
    fn sub_assign(&mut self, rhs: Pt) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl std::ops::Mul<f32> for Pt {
    type Output = Pt;
    fn mul(self, rhs: f32) -> Pt {
        let v = (self.0 as f32) * rhs;
        Pt::from(v)
    }
}

impl std::ops::Div<f32> for Pt {
    type Output = Pt;
    fn div(self, rhs: f32) -> Pt {
        let v = (self.0 as f32) / rhs;
        Pt::from(v)
    }
}

impl std::ops::MulAssign<f32> for Pt {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl std::ops::DivAssign<f32> for Pt {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
