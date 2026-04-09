#[cfg(feature = "gltf")]
pub mod gltf;
/// Helpers for bridging assets decoded by the `image` crate into `spottedcat::Image`.
#[cfg(feature = "utils")]
pub mod image;
#[cfg(feature = "model-3d")]
pub mod obj;
