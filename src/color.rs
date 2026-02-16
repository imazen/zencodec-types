//! Color profile types for CMS integration.
//!
//! Provides a unified way to reference the source color space of decoded
//! pixels, suitable for passing to a CMS backend (e.g., moxcms, lcms2).

use crate::Cicp;

/// A source color profile — either ICC bytes or CICP parameters.
///
/// This unified type lets consumers pass decoded image color info
/// directly to a CMS backend without caring whether the source had
/// an ICC profile, CICP codes, or a well-known named profile.
///
/// # Example
///
/// ```ignore
/// let source = decode_output.info().color_profile_source()
///     .unwrap_or(ColorProfileSource::Named(NamedProfile::Srgb));
/// let target = ColorProfileSource::Named(NamedProfile::LinearSrgb);
/// let transform = cms.create_transform(source, target, layout)?;
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorProfileSource<'a> {
    /// Raw ICC profile data.
    Icc(&'a [u8]),
    /// CICP parameters (a CMS can synthesize an equivalent profile).
    Cicp(Cicp),
    /// Well-known named profile.
    Named(NamedProfile),
}

/// Well-known color profiles that any CMS should recognize.
///
/// These cover the profiles encountered in practice for still images
/// and HDR content. A CMS backend maps each to the appropriate internal
/// representation (ICC profile, colorant matrix, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum NamedProfile {
    /// sRGB (IEC 61966-2-1). The web and desktop default.
    #[default]
    Srgb,
    /// Display P3 with sRGB transfer curve. Used by Apple displays, wide-gamut web content.
    DisplayP3,
    /// BT.2020 with BT.709 transfer (SDR wide gamut).
    Bt2020,
    /// BT.2020 with PQ transfer (HDR10, SMPTE ST 2084).
    Bt2020Pq,
    /// BT.2020 with HLG transfer (ARIB STD-B67, HDR broadcast).
    Bt2020Hlg,
    /// Adobe RGB (1998). Used in print workflows.
    AdobeRgb,
    /// Linear sRGB (sRGB primaries, gamma 1.0). Correct working space
    /// for alpha compositing and physically-based rendering.
    LinearSrgb,
}

impl NamedProfile {
    /// Convert to CICP parameters, if a standard mapping exists.
    ///
    /// Returns `None` for profiles without standard CICP codes (e.g., Adobe RGB).
    pub const fn to_cicp(self) -> Option<Cicp> {
        match self {
            Self::Srgb => Some(Cicp::SRGB),
            Self::DisplayP3 => Some(Cicp::DISPLAY_P3),
            Self::Bt2020 => Some(Cicp {
                color_primaries: 9,
                transfer_characteristics: 1,
                matrix_coefficients: 0,
                full_range: true,
            }),
            Self::Bt2020Pq => Some(Cicp::BT2100_PQ),
            Self::Bt2020Hlg => Some(Cicp::BT2100_HLG),
            Self::LinearSrgb => Some(Cicp {
                color_primaries: 1,
                transfer_characteristics: 8,
                matrix_coefficients: 0,
                full_range: true,
            }),
            Self::AdobeRgb => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_profile_default_is_srgb() {
        assert_eq!(NamedProfile::default(), NamedProfile::Srgb);
    }

    #[test]
    fn named_profile_to_cicp() {
        assert_eq!(NamedProfile::Srgb.to_cicp(), Some(Cicp::SRGB));
        assert_eq!(NamedProfile::Bt2020Pq.to_cicp(), Some(Cicp::BT2100_PQ));
        assert_eq!(NamedProfile::Bt2020Hlg.to_cicp(), Some(Cicp::BT2100_HLG));
        assert!(NamedProfile::AdobeRgb.to_cicp().is_none());
        assert!(NamedProfile::LinearSrgb.to_cicp().is_some());
        assert!(NamedProfile::DisplayP3.to_cicp().is_some());
    }

    #[test]
    fn color_profile_source_from_cicp() {
        let src = ColorProfileSource::Cicp(Cicp::SRGB);
        assert_eq!(src, ColorProfileSource::Cicp(Cicp::SRGB));
    }

    #[test]
    fn color_profile_source_from_icc() {
        let icc_data = [0u8; 16];
        let src = ColorProfileSource::Icc(&icc_data);
        if let ColorProfileSource::Icc(data) = src {
            assert_eq!(data.len(), 16);
        } else {
            panic!("expected Icc variant");
        }
    }

    #[test]
    fn color_profile_source_from_named() {
        let src = ColorProfileSource::Named(NamedProfile::DisplayP3);
        assert_eq!(src, ColorProfileSource::Named(NamedProfile::DisplayP3));
    }
}
