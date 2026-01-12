use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Debug, Clone, Copy, Default)]
pub struct Pt(pub(crate) f32);

impl Display for Pt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for Pt {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Pt {}

impl Hash for Pt {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state)
    }
}

impl PartialOrd for Pt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Pt {
    pub fn as_f32(self) -> f32 {
        self.0
    }

    pub fn as_i32(self) -> i32 {
        let v = if self.0.is_finite() { self.0 } else { 0.0 };
        v.round() as i32
    }

    pub(crate) fn to_u32_clamped(self) -> u32 {
        let v = if self.0.is_finite() { self.0 } else { 0.0 };
        if v <= 0.0 {
            0
        } else {
            let v = v.round().min(u32::MAX as f32);
            v as u32
        }
    }

    pub(crate) fn from_physical_px(px: f64, scale_factor: f64) -> Self {
        let v = px / scale_factor;
        let v = if v.is_finite() { v } else { 0.0 };
        let v = v.round();
        Pt(v as f32)
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
        let v = if value.is_finite() { value } else { 0.0 };
        Pt(v)
    }
}

impl From<f64> for Pt {
    fn from(value: f64) -> Self {
        let v = if value.is_finite() { value } else { 0.0 };
        Pt(v as f32)
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
        *self = *self * rhs;
    }
}

impl std::ops::DivAssign<f32> for Pt {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
