/// Style controls for the fullscreen fog background pass.
///
/// These values are intentionally artistic rather than physical:
/// they shape how the sky / horizon / lower background blend with `FogSettings::color`.
///
/// Use these when you want to push the fog toward a mood such as
/// "morning mist" or "dense atmosphere" without changing how fog accumulates in space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogBackgroundSettings {
    pub zenith_color: [f32; 3],
    pub zenith_mix: f32,
    pub horizon_color: [f32; 3],
    pub horizon_mix: f32,
    pub nadir_color: [f32; 3],
    pub nadir_mix: f32,
    pub horizon_glow_strength: f32,
    pub sky_fog_blend: f32,
    pub geometry_fog_blend: f32,
}

impl Default for FogBackgroundSettings {
    fn default() -> Self {
        Self {
            zenith_color: [0.27, 0.38, 0.52],
            zenith_mix: 0.38,
            horizon_color: [0.75, 0.79, 0.80],
            horizon_mix: 0.32,
            nadir_color: [0.52, 0.56, 0.55],
            nadir_mix: 0.18,
            horizon_glow_strength: 0.05,
            sky_fog_blend: 0.72,
            geometry_fog_blend: 0.55,
        }
    }
}

impl FogBackgroundSettings {
    /// A light, airy preset intended for clearer "morning mist" scenes.
    pub fn morning_mist() -> Self {
        Self::default()
    }

    /// A heavier preset intended for denser, moodier atmospheric scenes.
    pub fn dense_atmosphere() -> Self {
        Self {
            zenith_color: [0.18, 0.24, 0.32],
            zenith_mix: 0.50,
            horizon_color: [0.68, 0.66, 0.62],
            horizon_mix: 0.45,
            nadir_color: [0.44, 0.42, 0.40],
            nadir_mix: 0.28,
            horizon_glow_strength: 0.03,
            sky_fog_blend: 0.84,
            geometry_fog_blend: 0.72,
        }
    }

    /// Sets the upper-sky tint and its blend amount against the main fog color.
    pub fn with_zenith(mut self, color: [f32; 3], mix: f32) -> Self {
        self.zenith_color = color;
        self.zenith_mix = mix;
        self
    }

    /// Sets the horizon tint and its blend amount against the main fog color.
    pub fn with_horizon(mut self, color: [f32; 3], mix: f32) -> Self {
        self.horizon_color = color;
        self.horizon_mix = mix;
        self
    }

    /// Sets the lower-background tint and its blend amount against the main fog color.
    pub fn with_nadir(mut self, color: [f32; 3], mix: f32) -> Self {
        self.nadir_color = color;
        self.nadir_mix = mix;
        self
    }

    /// Adjusts the soft brightness boost around the horizon line.
    pub fn with_horizon_glow(mut self, strength: f32) -> Self {
        self.horizon_glow_strength = strength;
        self
    }

    /// Controls how strongly the background and fogged geometry lean toward the fog color.
    pub fn with_blend(mut self, sky_fog_blend: f32, geometry_fog_blend: f32) -> Self {
        self.sky_fog_blend = sky_fog_blend;
        self.geometry_fog_blend = geometry_fog_blend;
        self
    }
}

/// Quality / cost controls for the fullscreen fog pass.
///
/// These are not art-direction knobs. They tune sampling behavior and therefore
/// affect performance, stability, and how closely the height fog approximates
/// volumetric integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogSamplingSettings {
    pub min_height_samples: u32,
    pub max_height_samples: u32,
    pub height_sample_scale: f32,
}

impl Default for FogSamplingSettings {
    fn default() -> Self {
        Self {
            min_height_samples: 4,
            max_height_samples: 10,
            height_sample_scale: 0.6,
        }
    }
}

impl FogSamplingSettings {
    /// Sets the minimum and maximum sample count used by height fog integration.
    pub fn with_height_samples(mut self, min_height_samples: u32, max_height_samples: u32) -> Self {
        self.min_height_samples = min_height_samples;
        self.max_height_samples = max_height_samples;
        self
    }

    /// Controls how quickly sample count grows with travel distance.
    pub fn with_height_sample_scale(mut self, height_sample_scale: f32) -> Self {
        self.height_sample_scale = height_sample_scale;
        self
    }
}

/// Engine-level fog configuration.
///
/// `FogSettings` is split into three conceptual groups:
///
/// Physical controls:
/// - `with_strength`: overall fog intensity multiplier
/// - `with_distance`: how fog accumulates with camera-to-fragment distance
/// - `with_height`: how fog accumulates relative to world height
///
/// Style controls:
/// - `with_color`: the base fog color used for blending
/// - `with_background`: background / sky styling for the fullscreen fog pass
///
/// Quality controls:
/// - `with_sampling`: height-fog sampling quality and performance tradeoffs
///
/// Fog is disabled by default. It only becomes active once both of these are true:
/// - effective strength is greater than `0.0`
/// - either distance fog density or height fog density is greater than `0.0`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FogSettings {
    pub color: [f32; 4],
    pub strength: Option<f32>,
    pub background: FogBackgroundSettings,
    pub sampling: FogSamplingSettings,
    pub distance_start: f32,
    pub distance_end: f32,
    pub distance_exponent: f32,
    pub distance_density: f32,
    pub height_base: f32,
    pub height_falloff: f32,
    pub height_exponent: f32,
    pub height_density: f32,
}

impl Default for FogSettings {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 0.0],
            strength: None,
            background: FogBackgroundSettings::default(),
            sampling: FogSamplingSettings::default(),
            distance_start: 0.0,
            distance_end: 1.0,
            distance_exponent: 1.0,
            distance_density: 0.0,
            height_base: 0.0,
            height_falloff: 1.0,
            height_exponent: 1.0,
            height_density: 0.0,
        }
    }
}

impl FogSettings {
    /// Returns an explicitly disabled fog configuration.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Sets the base fog color.
    ///
    /// For backward compatibility, `color[3]` is still used as the fog strength
    /// when `with_strength(...)` is not provided.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Sets the overall fog intensity multiplier.
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = Some(strength);
        self
    }

    /// Applies artistic background styling for the fullscreen fog pass.
    pub fn with_background(mut self, background: FogBackgroundSettings) -> Self {
        self.background = background;
        self
    }

    /// Applies quality / performance controls for height fog integration.
    pub fn with_sampling(mut self, sampling: FogSamplingSettings) -> Self {
        self.sampling = sampling;
        self
    }

    /// Configures distance-based fog accumulation.
    ///
    /// Parameters:
    /// - `start`: distance where fog begins to ramp in
    /// - `end`: distance where the main ramp reaches full effect
    /// - `density`: contribution of the distance fog term
    /// - `exponent`: shape of the distance ramp
    pub fn with_distance(mut self, start: f32, end: f32, density: f32, exponent: f32) -> Self {
        self.distance_start = start;
        self.distance_end = end;
        self.distance_density = density;
        self.distance_exponent = exponent;
        self
    }

    /// Configures height-based fog accumulation.
    ///
    /// Parameters:
    /// - `base`: world-space height where the fog layer starts
    /// - `falloff`: vertical range over which the fog fades out
    /// - `density`: contribution of the height fog term
    /// - `exponent`: shape of the vertical density curve
    pub fn with_height(mut self, base: f32, falloff: f32, density: f32, exponent: f32) -> Self {
        self.height_base = base;
        self.height_falloff = falloff;
        self.height_density = density;
        self.height_exponent = exponent;
        self
    }

    #[cfg_attr(not(feature = "model-3d"), allow(dead_code))]
    pub(crate) fn effective_strength(&self) -> f32 {
        self.strength.unwrap_or(self.color[3])
    }

    #[cfg_attr(not(feature = "model-3d"), allow(dead_code))]
    pub(crate) fn is_enabled(&self) -> bool {
        self.effective_strength() > 0.0
            && (self.distance_density > 0.0 || self.height_density > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::FogSettings;

    #[test]
    fn fog_is_disabled_by_default() {
        let fog = FogSettings::default();
        assert!(!fog.is_enabled());
        assert_eq!(fog.effective_strength(), 0.0);
    }

    #[test]
    fn fog_requires_strength_and_density() {
        let no_density = FogSettings::default().with_strength(1.0);
        assert!(!no_density.is_enabled());

        let no_strength = FogSettings::default().with_distance(1.0, 10.0, 1.0, 1.0);
        assert!(!no_strength.is_enabled());

        let enabled = FogSettings::default()
            .with_strength(1.0)
            .with_distance(1.0, 10.0, 1.0, 1.0);
        assert!(enabled.is_enabled());
    }
}
