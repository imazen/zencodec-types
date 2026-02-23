//! Color profile types for CMS integration.
//!
//! Provides a unified way to reference the source color space of decoded
//! pixels, suitable for passing to a CMS backend (e.g., moxcms, lcms2).
//!
//! [`ColorContext`] bundles ICC and CICP metadata for cheap sharing
//! (via `Arc`) across pixel slices and pipeline stages.
//! [`WorkingColorSpace`] tracks which color space pixels are currently
//! in, so the planner knows what transforms have been applied.

use crate::Cicp;
use alloc::sync::Arc;

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
#[non_exhaustive]
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
#[non_exhaustive]
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

/// Color space metadata for pixel data.
///
/// Bundles ICC profile bytes and/or CICP parameters into a single
/// shareable context. Carried via `Arc` on [`PixelSlice`](crate::PixelSlice)
/// and pipeline sources so color metadata travels with pixel data
/// without per-strip cloning overhead.
///
/// Constructed from decoder output — see [`ImageInfo::color_context()`](crate::ImageInfo::color_context).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorContext {
    /// Raw ICC profile bytes — for CMS transforms or output embedding.
    pub icc: Option<Arc<[u8]>>,
    /// CICP parameters (ITU-T H.273).
    pub cicp: Option<Cicp>,
}

impl ColorContext {
    /// Create from an ICC profile.
    pub fn from_icc(icc: impl Into<Arc<[u8]>>) -> Self {
        Self {
            icc: Some(icc.into()),
            cicp: None,
        }
    }

    /// Create from CICP parameters.
    pub fn from_cicp(cicp: Cicp) -> Self {
        Self {
            icc: None,
            cicp: Some(cicp),
        }
    }

    /// Create from both ICC and CICP.
    pub fn from_icc_and_cicp(icc: impl Into<Arc<[u8]>>, cicp: Cicp) -> Self {
        Self {
            icc: Some(icc.into()),
            cicp: Some(cicp),
        }
    }

    /// Get a [`ColorProfileSource`] reference for CMS integration.
    ///
    /// Returns CICP if present (takes precedence per AVIF/HEIF specs),
    /// otherwise returns the ICC profile bytes.
    pub fn as_profile_source(&self) -> Option<ColorProfileSource<'_>> {
        if let Some(cicp) = self.cicp {
            Some(ColorProfileSource::Cicp(cicp))
        } else {
            self.icc.as_deref().map(ColorProfileSource::Icc)
        }
    }

    /// Derive transfer function from CICP (or `Unknown` if no CICP).
    pub fn transfer_function(&self) -> crate::TransferFunction {
        self.cicp
            .and_then(|c| crate::TransferFunction::from_cicp(c.transfer_characteristics))
            .unwrap_or(crate::TransferFunction::Unknown)
    }

    /// True if this describes sRGB (either via CICP or trivially).
    ///
    /// Note: this only checks CICP codes. A full sRGB ICC profile
    /// check would require parsing the ICC data.
    pub fn is_srgb(&self) -> bool {
        self.cicp == Some(Cicp::SRGB)
    }
}

/// What color space the pixels are currently in.
///
/// Tracked on [`PixelSlice`](crate::PixelSlice) and pipeline sources
/// to know what transforms have been applied. The planner uses this
/// to determine what conversions are still needed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WorkingColorSpace {
    /// Original decoded space. ICC/CICP NOT applied.
    /// Transfer function from [`PixelDescriptor`](crate::PixelDescriptor) applies.
    #[default]
    Native,
    /// Linear light, sRGB/BT.709 primaries.
    LinearSrgb,
    /// Linear light, BT.2020 primaries (wide gamut).
    LinearRec2020,
    /// Oklab perceptual uniform space (L, a, b).
    Oklab,
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

    // --- ColorContext tests ---

    #[test]
    fn color_context_from_icc() {
        let ctx = ColorContext::from_icc(alloc::vec![1, 2, 3]);
        assert!(ctx.icc.is_some());
        assert_eq!(ctx.icc.as_deref(), Some([1, 2, 3].as_slice()));
        assert!(ctx.cicp.is_none());
    }

    #[test]
    fn color_context_from_cicp() {
        let ctx = ColorContext::from_cicp(Cicp::SRGB);
        assert!(ctx.icc.is_none());
        assert_eq!(ctx.cicp, Some(Cicp::SRGB));
    }

    #[test]
    fn color_context_from_both() {
        let ctx = ColorContext::from_icc_and_cicp(alloc::vec![10, 20], Cicp::BT2100_PQ);
        assert!(ctx.icc.is_some());
        assert_eq!(ctx.cicp, Some(Cicp::BT2100_PQ));
    }

    #[test]
    fn color_context_profile_source_prefers_cicp() {
        let ctx = ColorContext::from_icc_and_cicp(alloc::vec![1, 2, 3], Cicp::SRGB);
        let src = ctx.as_profile_source().unwrap();
        assert_eq!(src, ColorProfileSource::Cicp(Cicp::SRGB));
    }

    #[test]
    fn color_context_profile_source_falls_back_to_icc() {
        let ctx = ColorContext::from_icc(alloc::vec![1, 2, 3]);
        let src = ctx.as_profile_source().unwrap();
        if let ColorProfileSource::Icc(data) = src {
            assert_eq!(data, &[1, 2, 3]);
        } else {
            panic!("expected Icc variant");
        }
    }

    #[test]
    fn color_context_is_srgb() {
        assert!(ColorContext::from_cicp(Cicp::SRGB).is_srgb());
        assert!(!ColorContext::from_cicp(Cicp::BT2100_PQ).is_srgb());
        assert!(!ColorContext::from_icc(alloc::vec![1, 2, 3]).is_srgb());
    }

    #[test]
    fn color_context_transfer_function() {
        use crate::TransferFunction;
        assert_eq!(
            ColorContext::from_cicp(Cicp::SRGB).transfer_function(),
            TransferFunction::Srgb
        );
        assert_eq!(
            ColorContext::from_icc(alloc::vec![1]).transfer_function(),
            TransferFunction::Unknown
        );
    }

    // --- WorkingColorSpace tests ---

    #[test]
    fn working_color_space_default() {
        assert_eq!(WorkingColorSpace::default(), WorkingColorSpace::Native);
    }

    #[test]
    fn working_color_space_eq() {
        assert_eq!(WorkingColorSpace::LinearSrgb, WorkingColorSpace::LinearSrgb);
        assert_ne!(WorkingColorSpace::Native, WorkingColorSpace::LinearSrgb);
    }
}
