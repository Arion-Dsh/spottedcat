//! Centralized 3D math utilities using ultraviolet internally.
//! All public APIs use standard Rust arrays to hide implementation details.

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
