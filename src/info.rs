//! Image metadata types.

use alloc::vec::Vec;

use crate::{ImageFormat, Orientation};

/// CICP color description (ITU-T H.273).
///
/// Coding-Independent Code Points describe the color space of an image
/// without requiring an ICC profile. Used by AVIF, HEIF, JPEG XL, and
/// video codecs (H.264, H.265, AV1).
///
/// Common combinations:
/// - sRGB: `(1, 13, 6, true)` — BT.709 primaries, sRGB transfer, BT.601 matrix
/// - Display P3: `(12, 16, 6, true)` — P3 primaries, PQ transfer
/// - BT.2100 PQ (HDR): `(9, 16, 9, true)` — BT.2020 primaries, PQ transfer
/// - BT.2100 HLG (HDR): `(9, 18, 9, true)` — BT.2020 primaries, HLG transfer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cicp {
    /// Color primaries (ColourPrimaries). Common values:
    /// 1 = BT.709/sRGB, 9 = BT.2020, 12 = Display P3.
    pub color_primaries: u8,
    /// Transfer characteristics (TransferCharacteristics). Common values:
    /// 1 = BT.709, 13 = sRGB, 16 = PQ (HDR), 18 = HLG (HDR).
    pub transfer_characteristics: u8,
    /// Matrix coefficients (MatrixCoefficients). Common values:
    /// 0 = Identity/RGB, 1 = BT.709, 6 = BT.601, 9 = BT.2020.
    pub matrix_coefficients: u8,
    /// Whether pixel values use the full range (0-255 for 8-bit)
    /// or video/limited range (16-235 for 8-bit luma).
    pub full_range: bool,
}

impl Cicp {
    /// sRGB color space: BT.709 primaries, sRGB transfer, BT.601 matrix, full range.
    pub const SRGB: Self = Self {
        color_primaries: 1,
        transfer_characteristics: 13,
        matrix_coefficients: 6,
        full_range: true,
    };

    /// BT.2100 PQ (HDR10): BT.2020 primaries, PQ transfer, BT.2020 matrix, full range.
    pub const BT2100_PQ: Self = Self {
        color_primaries: 9,
        transfer_characteristics: 16,
        matrix_coefficients: 9,
        full_range: true,
    };

    /// BT.2100 HLG (HDR): BT.2020 primaries, HLG transfer, BT.2020 matrix, full range.
    pub const BT2100_HLG: Self = Self {
        color_primaries: 9,
        transfer_characteristics: 18,
        matrix_coefficients: 9,
        full_range: true,
    };
}

/// Content Light Level Info (CEA-861.3).
///
/// Describes the light level of HDR content. Used alongside [`MasteringDisplay`]
/// to guide tone mapping on displays with different capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContentLightLevel {
    /// Maximum Content Light Level in cd/m² (nits).
    /// Peak luminance of any single pixel in the content.
    pub max_content_light_level: u16,
    /// Maximum Frame-Average Light Level in cd/m² (nits).
    /// Peak average luminance of any single frame.
    pub max_frame_average_light_level: u16,
}

/// Mastering Display Color Volume (SMPTE ST 2086).
///
/// Describes the color volume of the display used to master HDR content.
/// Chromaticity values are in units of 0.00002 (as per the spec).
/// Luminance values are in units of 0.0001 cd/m².
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MasteringDisplay {
    /// Display primaries chromaticity [R, G, B], each as [x, y].
    /// Values in units of 0.00002. (50000 = 1.0)
    pub primaries: [[u16; 2]; 3],
    /// White point chromaticity [x, y].
    /// Values in units of 0.00002. (50000 = 1.0)
    pub white_point: [u16; 2],
    /// Maximum display luminance in units of 0.0001 cd/m².
    pub max_luminance: u32,
    /// Minimum display luminance in units of 0.0001 cd/m².
    pub min_luminance: u32,
}

/// Image metadata obtained from probing or decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImageInfo {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Detected image format.
    pub format: ImageFormat,
    /// Whether the image has an alpha channel.
    pub has_alpha: bool,
    /// Whether the image contains animation (multiple frames).
    pub has_animation: bool,
    /// Number of frames (None if unknown without full parse).
    pub frame_count: Option<u32>,
    /// Bits per channel (e.g. 8, 10, 12, 16, 32).
    ///
    /// `None` if unknown (e.g. from a header-only probe that doesn't
    /// report bit depth).
    pub bit_depth: Option<u8>,
    /// Number of channels (1=gray, 2=gray+alpha, 3=RGB, 4=RGBA).
    ///
    /// `None` if unknown.
    pub channel_count: Option<u8>,
    /// CICP color description (ITU-T H.273).
    ///
    /// When present, describes the color space without requiring an ICC
    /// profile. Both CICP and ICC may be present — CICP takes precedence
    /// per AVIF/HEIF specs, but callers should use ICC when CICP is absent.
    pub cicp: Option<Cicp>,
    /// Content Light Level Info (CEA-861.3) for HDR content.
    pub content_light_level: Option<ContentLightLevel>,
    /// Mastering Display Color Volume (SMPTE ST 2086) for HDR content.
    pub mastering_display: Option<MasteringDisplay>,
    /// Embedded ICC color profile.
    pub icc_profile: Option<Vec<u8>>,
    /// Embedded EXIF metadata.
    pub exif: Option<Vec<u8>>,
    /// Embedded XMP metadata.
    pub xmp: Option<Vec<u8>>,
    /// EXIF orientation (1-8).
    ///
    /// When a codec applies orientation during decode (rotating the pixel
    /// data), this is set to [`Normal`](Orientation::Normal) and `width`/`height`
    /// reflect the display dimensions.
    ///
    /// When orientation is NOT applied, `width`/`height` are the stored
    /// dimensions and this field tells the caller what transform to apply.
    /// Use [`display_width()`](ImageInfo::display_width) /
    /// [`display_height()`](ImageInfo::display_height) to get effective
    /// display dimensions regardless.
    pub orientation: Orientation,
}

impl ImageInfo {
    /// Create a new `ImageInfo` with the given dimensions and format.
    ///
    /// Other fields default to no alpha, no animation, no metadata.
    /// Use the `with_*` builder methods to set them.
    pub fn new(width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            width,
            height,
            format,
            has_alpha: false,
            has_animation: false,
            frame_count: None,
            bit_depth: None,
            channel_count: None,
            cicp: None,
            content_light_level: None,
            mastering_display: None,
            icc_profile: None,
            exif: None,
            xmp: None,
            orientation: Orientation::Normal,
        }
    }

    /// Set whether the image has alpha.
    pub fn with_alpha(mut self, has_alpha: bool) -> Self {
        self.has_alpha = has_alpha;
        self
    }

    /// Set whether the image is animated.
    pub fn with_animation(mut self, has_animation: bool) -> Self {
        self.has_animation = has_animation;
        self
    }

    /// Set the frame count.
    pub fn with_frame_count(mut self, count: u32) -> Self {
        self.frame_count = Some(count);
        self
    }

    /// Set the bit depth (bits per channel).
    pub fn with_bit_depth(mut self, bit_depth: u8) -> Self {
        self.bit_depth = Some(bit_depth);
        self
    }

    /// Set the channel count.
    pub fn with_channel_count(mut self, channel_count: u8) -> Self {
        self.channel_count = Some(channel_count);
        self
    }

    /// Set the CICP color description.
    pub fn with_cicp(mut self, cicp: Cicp) -> Self {
        self.cicp = Some(cicp);
        self
    }

    /// Set the Content Light Level Info.
    pub fn with_content_light_level(mut self, clli: ContentLightLevel) -> Self {
        self.content_light_level = Some(clli);
        self
    }

    /// Set the Mastering Display Color Volume.
    pub fn with_mastering_display(mut self, mdcv: MasteringDisplay) -> Self {
        self.mastering_display = Some(mdcv);
        self
    }

    /// Set the ICC color profile.
    pub fn with_icc_profile(mut self, icc: Vec<u8>) -> Self {
        self.icc_profile = Some(icc);
        self
    }

    /// Set the EXIF metadata.
    pub fn with_exif(mut self, exif: Vec<u8>) -> Self {
        self.exif = Some(exif);
        self
    }

    /// Set the XMP metadata.
    pub fn with_xmp(mut self, xmp: Vec<u8>) -> Self {
        self.xmp = Some(xmp);
        self
    }

    /// Set the EXIF orientation.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Display width after applying EXIF orientation.
    ///
    /// For orientations 5-8 (90/270 rotation), this returns `height`.
    /// For orientations 1-4, this returns `width`.
    pub fn display_width(&self) -> u32 {
        if self.orientation.swaps_dimensions() {
            self.height
        } else {
            self.width
        }
    }

    /// Display height after applying EXIF orientation.
    ///
    /// For orientations 5-8 (90/270 rotation), this returns `width`.
    /// For orientations 1-4, this returns `height`.
    pub fn display_height(&self) -> u32 {
        if self.orientation.swaps_dimensions() {
            self.width
        } else {
            self.height
        }
    }

    /// Borrow embedded metadata for roundtrip encode.
    pub fn metadata(&self) -> ImageMetadata<'_> {
        ImageMetadata {
            icc_profile: self.icc_profile.as_deref(),
            exif: self.exif.as_deref(),
            xmp: self.xmp.as_deref(),
            cicp: self.cicp,
            content_light_level: self.content_light_level,
            mastering_display: self.mastering_display,
        }
    }
}

/// Borrowed view of image metadata (ICC/EXIF/XMP/CICP/HDR).
///
/// Used when encoding to preserve metadata from the source image.
/// Borrows from [`ImageInfo`] or user-provided slices. CICP and HDR
/// metadata are `Copy` types, so no borrowing needed for those.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImageMetadata<'a> {
    /// ICC color profile.
    pub icc_profile: Option<&'a [u8]>,
    /// EXIF metadata.
    pub exif: Option<&'a [u8]>,
    /// XMP metadata.
    pub xmp: Option<&'a [u8]>,
    /// CICP color description.
    pub cicp: Option<Cicp>,
    /// Content Light Level Info for HDR content.
    pub content_light_level: Option<ContentLightLevel>,
    /// Mastering Display Color Volume for HDR content.
    pub mastering_display: Option<MasteringDisplay>,
}

impl<'a> ImageMetadata<'a> {
    /// Create empty metadata.
    pub fn none() -> Self {
        Self::default()
    }

    /// Set the ICC color profile.
    pub fn with_icc(mut self, icc: &'a [u8]) -> Self {
        self.icc_profile = Some(icc);
        self
    }

    /// Set the EXIF metadata.
    pub fn with_exif(mut self, exif: &'a [u8]) -> Self {
        self.exif = Some(exif);
        self
    }

    /// Set the XMP metadata.
    pub fn with_xmp(mut self, xmp: &'a [u8]) -> Self {
        self.xmp = Some(xmp);
        self
    }

    /// Set the CICP color description.
    pub fn with_cicp(mut self, cicp: Cicp) -> Self {
        self.cicp = Some(cicp);
        self
    }

    /// Set the Content Light Level Info.
    pub fn with_content_light_level(mut self, clli: ContentLightLevel) -> Self {
        self.content_light_level = Some(clli);
        self
    }

    /// Set the Mastering Display Color Volume.
    pub fn with_mastering_display(mut self, mdcv: MasteringDisplay) -> Self {
        self.mastering_display = Some(mdcv);
        self
    }

    /// Whether any metadata is present.
    pub fn is_empty(&self) -> bool {
        self.icc_profile.is_none()
            && self.exif.is_none()
            && self.xmp.is_none()
            && self.cicp.is_none()
            && self.content_light_level.is_none()
            && self.mastering_display.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_roundtrip() {
        let info = ImageInfo::new(100, 200, ImageFormat::Jpeg)
            .with_frame_count(1)
            .with_icc_profile(alloc::vec![1, 2, 3])
            .with_exif(alloc::vec![4, 5])
            .with_cicp(Cicp::SRGB)
            .with_content_light_level(ContentLightLevel {
                max_content_light_level: 1000,
                max_frame_average_light_level: 400,
            });
        let meta = info.metadata();
        assert_eq!(meta.icc_profile, Some([1, 2, 3].as_slice()));
        assert_eq!(meta.exif, Some([4, 5].as_slice()));
        assert!(meta.xmp.is_none());
        assert_eq!(meta.cicp, Some(Cicp::SRGB));
        assert_eq!(
            meta.content_light_level.unwrap().max_content_light_level,
            1000
        );
        assert!(meta.mastering_display.is_none());
        assert!(!meta.is_empty());
    }

    #[test]
    fn metadata_empty() {
        let meta = ImageMetadata::none();
        assert!(meta.is_empty());
    }

    #[test]
    fn metadata_equality() {
        let a = ImageMetadata::none().with_icc(&[1, 2, 3]);
        let b = ImageMetadata::none().with_icc(&[1, 2, 3]);
        assert_eq!(a, b);

        let c = ImageMetadata::none().with_icc(&[4, 5]);
        assert_ne!(a, c);
    }

    #[test]
    fn display_dimensions_normal() {
        let info = ImageInfo::new(100, 200, ImageFormat::Jpeg);
        assert_eq!(info.display_width(), 100);
        assert_eq!(info.display_height(), 200);
    }

    #[test]
    fn display_dimensions_rotated() {
        let info =
            ImageInfo::new(100, 200, ImageFormat::Jpeg).with_orientation(Orientation::Rotate90);
        assert_eq!(info.display_width(), 200);
        assert_eq!(info.display_height(), 100);
    }

    #[test]
    fn display_dimensions_rotate180() {
        let info =
            ImageInfo::new(100, 200, ImageFormat::Jpeg).with_orientation(Orientation::Rotate180);
        // 180 does not swap dimensions
        assert_eq!(info.display_width(), 100);
        assert_eq!(info.display_height(), 200);
    }

    #[test]
    fn display_dimensions_all_orientations() {
        let info = ImageInfo::new(100, 200, ImageFormat::Jpeg);
        for orient in [
            Orientation::Normal,
            Orientation::FlipHorizontal,
            Orientation::Rotate180,
            Orientation::FlipVertical,
        ] {
            let i = info.clone().with_orientation(orient);
            assert_eq!((i.display_width(), i.display_height()), (100, 200));
        }
        for orient in [
            Orientation::Transpose,
            Orientation::Rotate90,
            Orientation::Transverse,
            Orientation::Rotate270,
        ] {
            let i = info.clone().with_orientation(orient);
            assert_eq!((i.display_width(), i.display_height()), (200, 100));
        }
    }

    #[test]
    fn image_info_builder() {
        let info = ImageInfo::new(10, 20, ImageFormat::Png)
            .with_alpha(true)
            .with_animation(true)
            .with_frame_count(5)
            .with_icc_profile(alloc::vec![1, 2])
            .with_exif(alloc::vec![3, 4])
            .with_xmp(alloc::vec![5, 6]);
        assert!(info.has_alpha);
        assert!(info.has_animation);
        assert_eq!(info.frame_count, Some(5));
        assert_eq!(info.icc_profile.as_deref(), Some([1, 2].as_slice()));
        assert_eq!(info.exif.as_deref(), Some([3, 4].as_slice()));
        assert_eq!(info.xmp.as_deref(), Some([5, 6].as_slice()));
    }

    #[test]
    fn image_info_eq() {
        let a = ImageInfo::new(10, 20, ImageFormat::Png).with_alpha(true);
        let b = ImageInfo::new(10, 20, ImageFormat::Png).with_alpha(true);
        assert_eq!(a, b);

        let c = ImageInfo::new(10, 20, ImageFormat::Jpeg).with_alpha(true);
        assert_ne!(a, c);
    }

    #[test]
    fn cicp_constants() {
        assert_eq!(Cicp::SRGB.color_primaries, 1);
        assert_eq!(Cicp::SRGB.transfer_characteristics, 13);
        assert_eq!(Cicp::BT2100_PQ.transfer_characteristics, 16);
        assert_eq!(Cicp::BT2100_HLG.transfer_characteristics, 18);
        assert!(Cicp::SRGB.full_range);
    }

    #[test]
    fn image_info_bit_depth_channels() {
        let info = ImageInfo::new(100, 100, ImageFormat::Avif)
            .with_bit_depth(10)
            .with_channel_count(4)
            .with_alpha(true);
        assert_eq!(info.bit_depth, Some(10));
        assert_eq!(info.channel_count, Some(4));
    }

    #[test]
    fn image_info_hdr_metadata() {
        let clli = ContentLightLevel {
            max_content_light_level: 4000,
            max_frame_average_light_level: 1000,
        };
        let mdcv = MasteringDisplay {
            primaries: [[34000, 16000], [13250, 34500], [7500, 3000]],
            white_point: [15635, 16450],
            max_luminance: 40000000,
            min_luminance: 50,
        };
        let info = ImageInfo::new(3840, 2160, ImageFormat::Avif)
            .with_cicp(Cicp::BT2100_PQ)
            .with_content_light_level(clli)
            .with_mastering_display(mdcv);
        assert_eq!(info.cicp, Some(Cicp::BT2100_PQ));
        assert_eq!(
            info.content_light_level.unwrap().max_content_light_level,
            4000
        );
        assert_eq!(info.mastering_display.unwrap().max_luminance, 40000000);
    }

    #[test]
    fn metadata_with_cicp_not_empty() {
        let meta = ImageMetadata::none().with_cicp(Cicp::SRGB);
        assert!(!meta.is_empty());
    }

    #[test]
    fn metadata_with_hdr_not_empty() {
        let meta = ImageMetadata::none().with_content_light_level(ContentLightLevel {
            max_content_light_level: 1000,
            max_frame_average_light_level: 400,
        });
        assert!(!meta.is_empty());
    }
}
