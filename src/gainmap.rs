//! Gain map metadata per ISO 21496-1.
//!
//! A gain map describes how to combine a base image (SDR or HDR) with a
//! secondary gain map image to produce a rendering adapted to any display
//! capability. The metadata here describes the mathematical relationship;
//! the gain map pixel data is a separate image (typically grayscale, often
//! at lower resolution than the base).
//!
//! Supported by JPEG (UltraHDR), AVIF/HEIF (`tmap` derived item), and
//! JPEG XL (gain map bundle). The metadata format is interoperable across
//! all three — ISO 21496-1 standardizes it.
//!
//! # Usage
//!
//! Codecs that decode gain maps put `(PixelData, GainMapMetadata)` (or a
//! codec-specific wrapper) in [`DecodeOutput::extras`](crate::DecodeOutput).
//! Callers retrieve it via downcast. Structured trait methods for gain maps
//! may be added in a future version after the pattern is proven across
//! multiple codecs.

/// Gain map metadata per ISO 21496-1.
///
/// Describes how to combine a base image with a gain map image to produce
/// an HDR or SDR rendering at any display capability level.
///
/// All per-channel fields use `[f32; 3]` for R, G, B. When the gain map
/// uses a single value for all channels, set all three elements to the
/// same value.
///
/// # Reconstruction formula
///
/// ```text
/// recovery = gain_map_pixel / max_value   (normalized to 0..1)
/// log_recovery = pow(recovery, 1.0 / gamma)
/// weight = clamp((log2(display_boost) - hdr_capacity_min)
///                / (hdr_capacity_max - hdr_capacity_min), 0, 1)
/// log_boost = gain_map_min * (1 - log_recovery) + gain_map_max * log_recovery
/// output = (base + offset_sdr) * exp2(log_boost * weight) - offset_hdr
/// ```
///
/// The `weight` adapts continuously based on display capability — no
/// binary HDR-or-SDR switch.
///
/// # Example
///
/// ```
/// use zencodec_types::GainMapMetadata;
///
/// // Typical UltraHDR gain map: SDR base, boost up to 4x (2 stops)
/// let meta = GainMapMetadata {
///     gain_map_max: [2.0, 2.0, 2.0],  // log2(4.0) = 2.0
///     hdr_capacity_max: 2.0,
///     ..GainMapMetadata::default()
/// };
///
/// assert!(!meta.base_rendition_is_hdr);
/// assert_eq!(meta.gamma, [1.0, 1.0, 1.0]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainMapMetadata {
    /// Whether the base rendition is HDR.
    ///
    /// - `false` (default): base image is SDR, gain map boosts to HDR.
    /// - `true`: base image is HDR, gain map tone-maps down to SDR.
    pub base_rendition_is_hdr: bool,

    /// `log2(min_content_boost)` per channel.
    ///
    /// Minimum boost applied when the gain map pixel is 0.
    /// Default: `[0.0, 0.0, 0.0]` (no boost at minimum).
    pub gain_map_min: [f32; 3],

    /// `log2(max_content_boost)` per channel.
    ///
    /// Maximum boost applied when the gain map pixel is 1.
    /// No meaningful default — set this from the file's metadata.
    pub gain_map_max: [f32; 3],

    /// Encoding gamma per channel.
    ///
    /// Applied to gain map pixel values before interpolation.
    /// Default: `[1.0, 1.0, 1.0]` (linear).
    pub gamma: [f32; 3],

    /// SDR offset per channel.
    ///
    /// Added to the base SDR value before applying the boost.
    /// Prevents division by zero in the reconstruction formula.
    /// Default: `[1.0/64.0; 3]`.
    pub offset_sdr: [f32; 3],

    /// HDR offset per channel.
    ///
    /// Subtracted from the result after applying the boost.
    /// Default: `[1.0/64.0; 3]`.
    pub offset_hdr: [f32; 3],

    /// Minimum HDR headroom (log2 of display boost).
    ///
    /// Below this display capability, the gain map has no effect
    /// (weight = 0, output = base image).
    /// Default: `0.0`.
    pub hdr_capacity_min: f32,

    /// Maximum HDR headroom (log2 of display boost).
    ///
    /// At or above this display capability, the gain map fully applies
    /// (weight = 1). No meaningful default — set from file metadata.
    pub hdr_capacity_max: f32,

    /// Whether the gain map is in the base image's color space.
    ///
    /// When `false`, the gain map uses its own color space (described
    /// by the alternate rendition's color metadata).
    /// Default: `false`.
    pub use_base_color_space: bool,
}

impl Default for GainMapMetadata {
    fn default() -> Self {
        Self {
            base_rendition_is_hdr: false,
            gain_map_min: [0.0; 3],
            gain_map_max: [0.0; 3],
            gamma: [1.0; 3],
            offset_sdr: [1.0 / 64.0; 3],
            offset_hdr: [1.0 / 64.0; 3],
            hdr_capacity_min: 0.0,
            hdr_capacity_max: 0.0,
            use_base_color_space: false,
        }
    }
}

impl GainMapMetadata {
    /// Create metadata with ISO 21496-1 defaults.
    ///
    /// `gain_map_max` and `hdr_capacity_max` default to 0.0 (no effect).
    /// Set them from the file's metadata to get a useful gain map.
    pub const fn new() -> Self {
        Self {
            base_rendition_is_hdr: false,
            gain_map_min: [0.0, 0.0, 0.0],
            gain_map_max: [0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0],
            offset_sdr: [1.0 / 64.0, 1.0 / 64.0, 1.0 / 64.0],
            offset_hdr: [1.0 / 64.0, 1.0 / 64.0, 1.0 / 64.0],
            hdr_capacity_min: 0.0,
            hdr_capacity_max: 0.0,
            use_base_color_space: false,
        }
    }

    /// Whether all per-channel fields use the same value across R, G, B.
    ///
    /// When `true`, the gain map is a single-channel (grayscale) map
    /// applied uniformly to all color channels. This is the common case.
    pub fn is_uniform(&self) -> bool {
        Self::channels_equal(self.gain_map_min)
            && Self::channels_equal(self.gain_map_max)
            && Self::channels_equal(self.gamma)
            && Self::channels_equal(self.offset_sdr)
            && Self::channels_equal(self.offset_hdr)
    }

    fn channels_equal(v: [f32; 3]) -> bool {
        v[0] == v[1] && v[1] == v[2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let m = GainMapMetadata::default();
        assert!(!m.base_rendition_is_hdr);
        assert_eq!(m.gain_map_min, [0.0, 0.0, 0.0]);
        assert_eq!(m.gain_map_max, [0.0, 0.0, 0.0]);
        assert_eq!(m.gamma, [1.0, 1.0, 1.0]);
        assert_eq!(m.hdr_capacity_min, 0.0);
        assert_eq!(m.hdr_capacity_max, 0.0);
        assert!(!m.use_base_color_space);
        // offset defaults: 1/64
        assert!((m.offset_sdr[0] - 1.0 / 64.0).abs() < f32::EPSILON);
        assert!((m.offset_hdr[0] - 1.0 / 64.0).abs() < f32::EPSILON);
    }

    #[test]
    fn const_new_matches_default() {
        let a = GainMapMetadata::new();
        let b = GainMapMetadata::default();
        assert_eq!(a, b);
    }

    #[test]
    fn is_uniform_default() {
        let m = GainMapMetadata::default();
        assert!(m.is_uniform());
    }

    #[test]
    fn is_uniform_per_channel() {
        let m = GainMapMetadata {
            gain_map_max: [1.0, 2.0, 1.5],
            ..GainMapMetadata::default()
        };
        assert!(!m.is_uniform());
    }

    #[test]
    fn typical_ultrahdr() {
        let m = GainMapMetadata {
            gain_map_max: [2.0, 2.0, 2.0],
            hdr_capacity_max: 2.0,
            ..GainMapMetadata::default()
        };
        assert!(!m.base_rendition_is_hdr);
        assert!(m.is_uniform());
        assert_eq!(m.gain_map_max, [2.0; 3]);
        assert_eq!(m.hdr_capacity_max, 2.0);
    }

    #[test]
    fn hdr_base_tonemap_down() {
        let m = GainMapMetadata {
            base_rendition_is_hdr: true,
            gain_map_max: [3.0, 3.0, 3.0],
            hdr_capacity_max: 3.0,
            ..GainMapMetadata::default()
        };
        assert!(m.base_rendition_is_hdr);
        assert!(m.is_uniform());
    }

    #[test]
    fn copy_semantics() {
        let a = GainMapMetadata {
            gain_map_max: [2.0; 3],
            hdr_capacity_max: 2.0,
            ..GainMapMetadata::default()
        };
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
