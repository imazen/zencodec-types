//! Image metadata types.

use alloc::vec::Vec;

use crate::{ImageFormat, Orientation};

/// Image metadata obtained from probing or decoding.
#[derive(Clone, Debug, PartialEq)]
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
        }
    }
}

/// Borrowed view of image metadata (ICC/EXIF/XMP).
///
/// Used when encoding to preserve metadata from the source image.
/// Borrows from [`ImageInfo`] or user-provided slices.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ImageMetadata<'a> {
    /// ICC color profile.
    pub icc_profile: Option<&'a [u8]>,
    /// EXIF metadata.
    pub exif: Option<&'a [u8]>,
    /// XMP metadata.
    pub xmp: Option<&'a [u8]>,
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

    /// Whether any metadata is present.
    pub fn is_empty(&self) -> bool {
        self.icc_profile.is_none() && self.exif.is_none() && self.xmp.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_roundtrip() {
        let info = ImageInfo {
            width: 100,
            height: 200,
            format: ImageFormat::Jpeg,
            has_alpha: false,
            has_animation: false,
            frame_count: Some(1),
            icc_profile: Some(alloc::vec![1, 2, 3]),
            exif: Some(alloc::vec![4, 5]),
            xmp: None,
            orientation: Orientation::Normal,
        };
        let meta = info.metadata();
        assert_eq!(meta.icc_profile, Some([1, 2, 3].as_slice()));
        assert_eq!(meta.exif, Some([4, 5].as_slice()));
        assert!(meta.xmp.is_none());
        assert!(!meta.is_empty());
    }

    #[test]
    fn metadata_empty() {
        let meta = ImageMetadata::none();
        assert!(meta.is_empty());
    }
}
