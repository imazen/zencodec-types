//! Encode and decode output types.

use alloc::vec::Vec;
use imgref::ImgVec;
use rgb::{Rgb, Rgba};

use crate::{ImageFormat, ImageInfo, ImageMetadata, PixelData};

/// Output from an encode operation.
#[derive(Clone, Debug)]
pub struct EncodeOutput {
    data: Vec<u8>,
    format: ImageFormat,
}

impl EncodeOutput {
    /// Create a new encode output.
    pub fn new(data: Vec<u8>, format: ImageFormat) -> Self {
        Self { data, format }
    }

    /// Consume and return the encoded bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// Borrow the encoded bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// Encoded byte count.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the output is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The format that was used for encoding.
    pub fn format(&self) -> ImageFormat {
        self.format
    }
}

impl AsRef<[u8]> for EncodeOutput {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

/// Output from a decode operation.
pub struct DecodeOutput {
    pixels: PixelData,
    info: ImageInfo,
}

impl DecodeOutput {
    /// Create a new decode output.
    pub fn new(pixels: PixelData, info: ImageInfo) -> Self {
        Self { pixels, info }
    }

    /// Borrow the pixel data in its native format.
    pub fn pixels(&self) -> &PixelData {
        &self.pixels
    }

    /// Take the pixel data, consuming this output.
    pub fn into_pixels(self) -> PixelData {
        self.pixels
    }

    /// Convert to RGB8, consuming this output.
    pub fn into_rgb8(self) -> ImgVec<Rgb<u8>> {
        self.pixels.into_rgb8()
    }

    /// Convert to RGBA8, consuming this output.
    pub fn into_rgba8(self) -> ImgVec<Rgba<u8>> {
        self.pixels.into_rgba8()
    }

    /// Borrow as RGB8 if that's the native format.
    pub fn as_rgb8(&self) -> Option<imgref::ImgRef<'_, Rgb<u8>>> {
        match &self.pixels {
            PixelData::Rgb8(img) => Some(img.as_ref()),
            _ => None,
        }
    }

    /// Borrow as RGBA8 if that's the native format.
    pub fn as_rgba8(&self) -> Option<imgref::ImgRef<'_, Rgba<u8>>> {
        match &self.pixels {
            PixelData::Rgba8(img) => Some(img.as_ref()),
            _ => None,
        }
    }

    /// Image info.
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }

    /// Image width.
    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    /// Image height.
    pub fn height(&self) -> u32 {
        self.pixels.height()
    }

    /// Whether the image has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        self.pixels.has_alpha()
    }

    /// Detected format.
    pub fn format(&self) -> ImageFormat {
        self.info.format
    }

    /// Borrow embedded metadata for roundtrip encode.
    pub fn metadata(&self) -> ImageMetadata<'_> {
        self.info.metadata()
    }
}

impl core::fmt::Debug for DecodeOutput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DecodeOutput")
            .field("pixels", &self.pixels)
            .field("format", &self.info.format)
            .finish()
    }
}

/// A single frame from animation decoding.
pub struct DecodeFrame {
    pixels: PixelData,
    delay_ms: u32,
    index: u32,
}

impl DecodeFrame {
    /// Create a new decode frame.
    pub fn new(pixels: PixelData, delay_ms: u32, index: u32) -> Self {
        Self {
            pixels,
            delay_ms,
            index,
        }
    }

    /// Borrow the pixel data.
    pub fn pixels(&self) -> &PixelData {
        &self.pixels
    }

    /// Take the pixel data, consuming this frame.
    pub fn into_pixels(self) -> PixelData {
        self.pixels
    }

    /// Convert to RGB8, consuming this frame.
    pub fn into_rgb8(self) -> ImgVec<Rgb<u8>> {
        self.pixels.into_rgb8()
    }

    /// Convert to RGBA8, consuming this frame.
    pub fn into_rgba8(self) -> ImgVec<Rgba<u8>> {
        self.pixels.into_rgba8()
    }

    /// Frame delay in milliseconds.
    pub fn delay_ms(&self) -> u32 {
        self.delay_ms
    }

    /// Frame index (0-based).
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Frame width.
    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    /// Frame height.
    pub fn height(&self) -> u32 {
        self.pixels.height()
    }
}

impl core::fmt::Debug for DecodeFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DecodeFrame")
            .field("pixels", &self.pixels)
            .field("delay_ms", &self.delay_ms)
            .field("index", &self.index)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn encode_output() {
        let output = EncodeOutput::new(vec![1, 2, 3], ImageFormat::Jpeg);
        assert_eq!(output.format(), ImageFormat::Jpeg);
        assert_eq!(output.len(), 3);
        assert_eq!(output.bytes(), &[1, 2, 3]);
        assert!(!output.is_empty());
        assert_eq!(output.into_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn decode_output() {
        let img = ImgVec::new(
            vec![
                Rgb {
                    r: 10u8,
                    g: 20,
                    b: 30
                };
                4
            ],
            2,
            2,
        );
        let info = ImageInfo {
            width: 2,
            height: 2,
            format: ImageFormat::Png,
            has_alpha: false,
            has_animation: false,
            frame_count: Some(1),
            icc_profile: None,
            exif: None,
            xmp: None,
        };
        let output = DecodeOutput::new(PixelData::Rgb8(img), info);
        assert_eq!(output.width(), 2);
        assert_eq!(output.height(), 2);
        assert!(!output.has_alpha());
        assert_eq!(output.format(), ImageFormat::Png);
        assert!(output.as_rgb8().is_some());
        assert!(output.as_rgba8().is_none());
    }

    #[test]
    fn decode_frame() {
        let img = ImgVec::new(
            vec![
                Rgba {
                    r: 1u8,
                    g: 2,
                    b: 3,
                    a: 4
                };
                4
            ],
            2,
            2,
        );
        let frame = DecodeFrame::new(PixelData::Rgba8(img), 100, 0);
        assert_eq!(frame.delay_ms(), 100);
        assert_eq!(frame.index(), 0);
        assert_eq!(frame.width(), 2);
        assert_eq!(frame.height(), 2);
    }
}
