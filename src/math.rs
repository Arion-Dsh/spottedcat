//! Centralized 3D math utilities using ultraviolet internally.
//! All public APIs use standard Rust arrays to hide implementation details.

use ultraviolet::Slerp;

/// 4x4 Matrix operations.
pub mod mat4 {
    /// Returns a 4x4 identity matrix.
    pub fn identity() -> [[f32; 4]; 4] {
        ultraviolet::Mat4::identity().into()
    }

    /// Creates a 4x4 translation matrix.
    pub fn from_translation(pos: [f32; 3]) -> [[f32; 4]; 4] {
        ultraviolet::Mat4::from_translation(pos.into()).into()
    }

    /// Creates a 4x4 non-uniform scale matrix.
    pub fn from_scale(scale: [f32; 3]) -> [[f32; 4]; 4] {
        ultraviolet::Mat4::from_nonuniform_scale(scale.into()).into()
    }

    /// Creates a 4x4 rotation matrix from Euler angles (Roll, Pitch, Yaw).
    pub fn from_rotation(rot: [f32; 3]) -> [[f32; 4]; 4] {
        let rx = ultraviolet::Mat4::from_rotation_x(rot[0]);
        let ry = ultraviolet::Mat4::from_rotation_y(rot[1]);
        let rz = ultraviolet::Mat4::from_rotation_z(rot[2]);
        (rx * ry * rz).into()
    }

    /// Creates a 4x4 rotation matrix from a quaternion [x, y, z, w].
    pub fn from_quat(q: [f32; 4]) -> [[f32; 4]; 4] {
        let rotor = ultraviolet::Rotor3::from_quaternion_array(q);
        rotor.into_matrix().into_homogeneous().into()
    }

    /// Multiplies two 4x4 matrices (A * B).
    pub fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let ma = ultraviolet::Mat4::from(a);
        let mb = ultraviolet::Mat4::from(b);
        (ma * mb).into()
    }

    /// Creates a Right-Handed 'look-at' matrix.
    pub fn look_at(eye: [f32; 3], at: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
        ultraviolet::Mat4::look_at(eye.into(), at.into(), up.into()).into()
    }
}

/// Perspective and Orthographic projections.
pub mod projection {
    /// Creates a Right-Handed perspective projection matrix for WGPU (0..1 depth).
    /// Uses degrees for fov_y.
    pub fn perspective_degrees(fov_y_deg: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
        let fov_y_rad = fov_y_deg.to_radians();
        ultraviolet::projection::perspective_wgpu_dx(fov_y_rad, aspect, near, far).into()
    }

    /// Creates a Right-Handed perspective projection matrix for WGPU (0..1 depth).
    /// Uses radians for fov_y.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
        ultraviolet::projection::perspective_wgpu_dx(fov_y, aspect, near, far).into()
    }
}

/// Trait for types that can be linearly interpolated.
pub trait Interpolatable {
    /// Linearly interpolate between `self` and `other` using `alpha` (0.0 to 1.0).
    fn interpolate(self, other: Self, alpha: f32) -> Self;
}

impl Interpolatable for f32 {
    fn interpolate(self, other: Self, alpha: f32) -> Self {
        self + (other - self) * alpha
    }
}

impl Interpolatable for [f32; 2] {
    fn interpolate(self, other: Self, alpha: f32) -> Self {
        [
            self[0] + (other[0] - self[0]) * alpha,
            self[1] + (other[1] - self[1]) * alpha,
        ]
    }
}

impl Interpolatable for [f32; 3] {
    fn interpolate(self, other: Self, alpha: f32) -> Self {
        [
            self[0] + (other[0] - self[0]) * alpha,
            self[1] + (other[1] - self[1]) * alpha,
            self[2] + (other[2] - self[2]) * alpha,
        ]
    }
}

impl Interpolatable for [f32; 4] {
    fn interpolate(self, other: Self, alpha: f32) -> Self {
        [
            self[0] + (other[0] - self[0]) * alpha,
            self[1] + (other[1] - self[1]) * alpha,
            self[2] + (other[2] - self[2]) * alpha,
            self[3] + (other[3] - self[3]) * alpha,
        ]
    }
}

/// A wrapper that simplifies state interpolation between fixed logic updates.
///
/// It keeps track of the 'previous' and 'current' values and automatically
/// performs linear interpolation when requested.
///
/// ### Example
///
/// ```rust
/// use spottedcat::{Context, DrawOption, Image, Pt, Spot};
/// use spottedcat::math::Interpolated;
///
/// struct Player {
///     pos: Interpolated<[f32; 2]>,
///     sprite: Image,
/// }
///
/// impl Spot for Player {
///     fn initialize(ctx: &mut Context) -> Self {
///         let sprite = Image::new(ctx, Pt::from(1.0), Pt::from(1.0), &[255, 255, 255, 255])
///             .unwrap();
///         Self {
///             pos: Interpolated::new([0.0, 0.0]),
///             sprite,
///         }
///     }
///
///     fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {
///         // Update the target value. Previous becomes current.
///         let next_x = self.pos.target()[0] + 1.0;
///         self.pos.update([next_x, 0.0]);
///     }
///
///     fn draw(&mut self, ctx: &mut Context, screen: Image) {
///         // Read the smoothly interpolated value for the current render frame
///         let pos = self.pos.value(ctx);
///         screen.draw(
///             ctx,
///             &self.sprite,
///             DrawOption::default().with_position([Pt::from(pos[0]), Pt::from(pos[1])]),
///         );
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Interpolated<T> {
    current: T,
    previous: T,
}

/// A quaternion representing a 3D rotation [x, y, z, w].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Quat(pub [f32; 4]);

impl Quat {
    /// Returns an identity quaternion (no rotation).
    pub fn identity() -> Self {
        Self([0.0, 0.0, 0.0, 1.0])
    }
}

impl From<[f32; 4]> for Quat {
    fn from(val: [f32; 4]) -> Self {
        Self(val)
    }
}

impl Interpolatable for Quat {
    /// Spherical Linear Interpolation (SLERP) for quaternions.
    fn interpolate(self, other: Self, alpha: f32) -> Self {
        let q1 = ultraviolet::Rotor3::from_quaternion_array(self.0);
        let q2 = ultraviolet::Rotor3::from_quaternion_array(other.0);
        let result = q1.slerp(q2, alpha);
        Self(result.into_quaternion_array())
    }
}

impl<T: Interpolatable + Copy> Interpolated<T> {
    /// Creates a new interpolated value.
    pub fn new(val: T) -> Self {
        Self {
            current: val,
            previous: val,
        }
    }

    /// Updates the value. The current value becomes the previous value.
    pub fn update(&mut self, next: T) {
        self.previous = self.current;
        self.current = next;
    }

    /// Sets both current and previous to the same value (teleport).
    pub fn teleport(&mut self, val: T) {
        self.current = val;
        self.previous = val;
    }

    /// Returns the raw current target value.
    pub fn target(&self) -> T {
        self.current
    }

    /// Returns the raw previous value.
    pub fn previous(&self) -> T {
        self.previous
    }

    /// Returns the interpolated value based on the current context's draw interpolation.
    pub fn value(&self, ctx: &crate::Context) -> T {
        self.previous
            .interpolate(self.current, ctx.draw_interpolation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolation_f32() {
        let mut val = Interpolated::new(0.0);
        val.update(10.0);
        assert_eq!(val.previous(), 0.0);
        assert_eq!(val.target(), 10.0);

        // Manual check of the trait
        assert_eq!(val.previous().interpolate(val.target(), 0.5), 5.0);
    }

    #[test]
    fn test_interpolation_v2() {
        let mut val = Interpolated::new([0.0, 0.0]);
        val.update([10.0, 20.0]);
        assert_eq!(val.previous().interpolate(val.target(), 0.5), [5.0, 10.0]);
    }

    #[test]
    fn test_interpolation_quat() {
        let q1 = Quat::identity();
        // 90 degree rotation around Z
        let q2 = Quat([0.0, 0.0, 0.70710677, 0.70710677]);

        let mid = q1.interpolate(q2, 0.5);
        // Should be roughly 45 degrees
        assert!(mid.0[2] > 0.0 && mid.0[2] < 0.7071);
        assert!(mid.0[3] > 0.7071);
    }
}
