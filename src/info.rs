//! Image metadata types.

use alloc::vec::Vec;

use crate::ImageFormat;

/// Image metadata obtained from probing or decoding.
#[derive(Clone, Debug)]
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
}

impl ImageInfo {
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
