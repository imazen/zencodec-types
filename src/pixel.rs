//! Typed pixel buffer definitions.
//!
//! Uses `imgref::ImgVec` for 2D pixel data with typed pixels from the `rgb` crate.
//!
//! `PixelData` is a data container — it holds pixels and describes their layout,
//! but does **not** convert between formats. Format conversion requires
//! transfer function awareness that belongs in a dedicated conversion crate.
//! Use [`Decoder::decode_into()`](crate::Decoder::decode_into) to request
//! a specific format from the codec, which can convert correctly.

use alloc::vec::Vec;
use imgref::ImgVec;
use rgb::alt::BGRA;
use rgb::{Gray, Rgb, Rgba};

/// Grayscale pixel with alpha channel.
///
/// A simple two-component pixel type. Not from the `rgb` crate — we own this
/// type to avoid API instability in `rgb::alt::GrayAlpha`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(C)]
pub struct GrayAlpha<T> {
    /// Gray value.
    pub v: T,
    /// Alpha value.
    pub a: T,
}

impl<T> GrayAlpha<T> {
    /// Create a new gray+alpha pixel.
    pub const fn new(v: T, a: T) -> Self {
        Self { v, a }
    }
}

/// Decoded pixel data in a typed buffer.
///
/// The variant determines both the pixel format and precision.
/// Width and height are embedded in the `ImgVec`.
///
/// # Transfer function
///
/// `PixelData` does not track its transfer function — that metadata lives in
/// [`ImageInfo::cicp`](crate::ImageInfo) (or the ICC profile). The actual
/// transfer function depends on how the data was produced (codec-specific).
///
/// If you need a specific pixel format, use
/// [`Decoder::decode_into()`](crate::Decoder::decode_into) to request it
/// from the codec. Codecs can convert correctly because they know the source
/// transfer function. Post-hoc conversion between depth classes (u8/u16 ↔ f32)
/// requires transfer function math that this crate intentionally does not provide.
///
/// u16 variants use the full 0–65535 range regardless of source bit depth
/// (e.g. 10-bit AVIF values are scaled up, not left in 0–1023).
#[non_exhaustive]
pub enum PixelData {
    /// 8-bit RGB.
    Rgb8(ImgVec<Rgb<u8>>),
    /// 8-bit RGBA.
    Rgba8(ImgVec<Rgba<u8>>),
    /// 16-bit RGB. Full 0–65535 range regardless of source bit depth.
    Rgb16(ImgVec<Rgb<u16>>),
    /// 16-bit RGBA. Full 0–65535 range regardless of source bit depth.
    Rgba16(ImgVec<Rgba<u16>>),
    /// RGB f32. Values in [0.0, 1.0] for SDR; may exceed 1.0 for HDR.
    RgbF32(ImgVec<Rgb<f32>>),
    /// RGBA f32. Values in [0.0, 1.0] for SDR; may exceed 1.0 for HDR.
    RgbaF32(ImgVec<Rgba<f32>>),
    /// 8-bit grayscale.
    Gray8(ImgVec<Gray<u8>>),
    /// 16-bit grayscale. Full 0–65535 range regardless of source bit depth.
    Gray16(ImgVec<Gray<u16>>),
    /// Grayscale f32. Values in [0.0, 1.0] for SDR; may exceed 1.0 for HDR.
    GrayF32(ImgVec<Gray<f32>>),
    /// 8-bit BGRA (blue, green, red, alpha byte order).
    ///
    /// Native byte order for Windows/DirectX surfaces.
    Bgra8(ImgVec<BGRA<u8>>),
    /// 8-bit grayscale with alpha.
    GrayAlpha8(ImgVec<GrayAlpha<u8>>),
    /// 16-bit grayscale with alpha. Full 0–65535 range regardless of source bit depth.
    GrayAlpha16(ImgVec<GrayAlpha<u16>>),
    /// Grayscale + alpha f32.
    GrayAlphaF32(ImgVec<GrayAlpha<f32>>),
}

impl PixelData {
    /// Pixel format descriptor for this variant.
    ///
    /// Returns a descriptor based on the channel type and layout. The transfer
    /// function is [`Unknown`](crate::TransferFunction::Unknown) because
    /// `PixelData` does not track its transfer function — that metadata lives
    /// in [`ImageInfo::cicp`](crate::ImageInfo) or the ICC profile.
    ///
    /// To get a descriptor with the correct transfer function, resolve it
    /// from CICP metadata:
    ///
    /// ```ignore
    /// let desc = pixels.descriptor();
    /// let tf = info.transfer_function(); // derives from CICP
    /// let resolved = desc.with_transfer(tf);
    /// ```
    pub fn descriptor(&self) -> crate::buffer::PixelDescriptor {
        use crate::buffer::PixelDescriptor;
        match self {
            PixelData::Rgb8(_) => PixelDescriptor::RGB8,
            PixelData::Rgba8(_) => PixelDescriptor::RGBA8,
            PixelData::Rgb16(_) => PixelDescriptor::RGB16,
            PixelData::Rgba16(_) => PixelDescriptor::RGBA16,
            PixelData::RgbF32(_) => PixelDescriptor::RGBF32,
            PixelData::RgbaF32(_) => PixelDescriptor::RGBAF32,
            PixelData::Gray8(_) => PixelDescriptor::GRAY8,
            PixelData::Gray16(_) => PixelDescriptor::GRAY16,
            PixelData::GrayF32(_) => PixelDescriptor::GRAYF32,
            PixelData::Bgra8(_) => PixelDescriptor::BGRA8,
            PixelData::GrayAlpha8(_) => PixelDescriptor::GRAYA8,
            PixelData::GrayAlpha16(_) => PixelDescriptor::GRAYA16,
            PixelData::GrayAlphaF32(_) => PixelDescriptor::GRAYAF32,
        }
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            PixelData::Rgb8(img) => img.width() as u32,
            PixelData::Rgba8(img) => img.width() as u32,
            PixelData::Rgb16(img) => img.width() as u32,
            PixelData::Rgba16(img) => img.width() as u32,
            PixelData::RgbF32(img) => img.width() as u32,
            PixelData::RgbaF32(img) => img.width() as u32,
            PixelData::Gray8(img) => img.width() as u32,
            PixelData::Gray16(img) => img.width() as u32,
            PixelData::GrayF32(img) => img.width() as u32,
            PixelData::Bgra8(img) => img.width() as u32,
            PixelData::GrayAlpha8(img) => img.width() as u32,
            PixelData::GrayAlpha16(img) => img.width() as u32,
            PixelData::GrayAlphaF32(img) => img.width() as u32,
        }
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            PixelData::Rgb8(img) => img.height() as u32,
            PixelData::Rgba8(img) => img.height() as u32,
            PixelData::Rgb16(img) => img.height() as u32,
            PixelData::Rgba16(img) => img.height() as u32,
            PixelData::RgbF32(img) => img.height() as u32,
            PixelData::RgbaF32(img) => img.height() as u32,
            PixelData::Gray8(img) => img.height() as u32,
            PixelData::Gray16(img) => img.height() as u32,
            PixelData::GrayF32(img) => img.height() as u32,
            PixelData::Bgra8(img) => img.height() as u32,
            PixelData::GrayAlpha8(img) => img.height() as u32,
            PixelData::GrayAlpha16(img) => img.height() as u32,
            PixelData::GrayAlphaF32(img) => img.height() as u32,
        }
    }

    /// Whether this pixel data has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            PixelData::Rgba8(_)
                | PixelData::Rgba16(_)
                | PixelData::RgbaF32(_)
                | PixelData::Bgra8(_)
                | PixelData::GrayAlpha8(_)
                | PixelData::GrayAlpha16(_)
                | PixelData::GrayAlphaF32(_)
        )
    }

    /// Borrow pixel data as a [`PixelSlice`](crate::buffer::PixelSlice).
    ///
    /// Returns `None` for GrayAlpha variants (our `GrayAlpha<T>` type
    /// doesn't implement `rgb::ComponentBytes`, so we can't get a byte
    /// slice without copying).
    pub fn as_pixel_slice(&self) -> Option<crate::buffer::PixelSlice<'_>> {
        // The From<ImgRef> impls use convention-based descriptors (sRGB for u8,
        // linear for f32). Override with self.descriptor() which preserves the
        // transfer-agnostic Unknown from decoded pixel data.
        let desc = self.descriptor();
        let slice: crate::buffer::PixelSlice<'_> = match self {
            PixelData::Rgb8(img) => img.as_ref().into(),
            PixelData::Rgba8(img) => img.as_ref().into(),
            PixelData::Rgb16(img) => img.as_ref().into(),
            PixelData::Rgba16(img) => img.as_ref().into(),
            PixelData::RgbF32(img) => img.as_ref().into(),
            PixelData::RgbaF32(img) => img.as_ref().into(),
            PixelData::Gray8(img) => img.as_ref().into(),
            PixelData::Gray16(img) => img.as_ref().into(),
            PixelData::GrayF32(img) => img.as_ref().into(),
            PixelData::Bgra8(img) => img.as_ref().into(),
            // GrayAlpha types don't implement ComponentBytes
            PixelData::GrayAlpha8(_) | PixelData::GrayAlpha16(_) | PixelData::GrayAlphaF32(_) => {
                return None;
            }
        };
        Some(slice.with_descriptor(desc))
    }

    /// Get the raw pixel data as a byte vector.
    ///
    /// Returns the raw bytes of the pixel buffer in its native format.
    /// No format conversion is performed.
    pub fn to_bytes(&self) -> Vec<u8> {
        use rgb::ComponentBytes;
        match self {
            PixelData::Rgb8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgba8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgb16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgba16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::RgbF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::RgbaF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Gray8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Gray16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::GrayF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Bgra8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.iter().flat_map(|p| [p.v, p.a]).collect()
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.iter()
                    .flat_map(|p| {
                        let mut b = [0u8; 4];
                        b[..2].copy_from_slice(&p.v.to_ne_bytes());
                        b[2..].copy_from_slice(&p.a.to_ne_bytes());
                        b
                    })
                    .collect()
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.iter()
                    .flat_map(|p| {
                        let mut b = [0u8; 8];
                        b[..4].copy_from_slice(&p.v.to_ne_bytes());
                        b[4..].copy_from_slice(&p.a.to_ne_bytes());
                        b
                    })
                    .collect()
            }
        }
    }
}

impl core::fmt::Debug for PixelData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variant = match self {
            PixelData::Rgb8(_) => "Rgb8",
            PixelData::Rgba8(_) => "Rgba8",
            PixelData::Rgb16(_) => "Rgb16",
            PixelData::Rgba16(_) => "Rgba16",
            PixelData::RgbF32(_) => "RgbF32",
            PixelData::RgbaF32(_) => "RgbaF32",
            PixelData::Gray8(_) => "Gray8",
            PixelData::Gray16(_) => "Gray16",
            PixelData::GrayF32(_) => "GrayF32",
            PixelData::Bgra8(_) => "Bgra8",
            PixelData::GrayAlpha8(_) => "GrayAlpha8",
            PixelData::GrayAlpha16(_) => "GrayAlpha16",
            PixelData::GrayAlphaF32(_) => "GrayAlphaF32",
        };
        write!(
            f,
            "PixelData::{}({}x{})",
            variant,
            self.width(),
            self.height()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn dimensions_and_alpha() {
        let img = ImgVec::new(vec![Rgb { r: 0u8, g: 0, b: 0 }; 100], 10, 10);
        let data = PixelData::Rgb8(img);
        assert_eq!(data.width(), 10);
        assert_eq!(data.height(), 10);
        assert!(!data.has_alpha());

        let data = PixelData::Rgba8(ImgVec::new(
            vec![
                Rgba {
                    r: 0u8,
                    g: 0,
                    b: 0,
                    a: 255
                };
                4
            ],
            2,
            2,
        ));
        assert!(data.has_alpha());

        let data = PixelData::Bgra8(ImgVec::new(
            vec![
                BGRA {
                    b: 0,
                    g: 0,
                    r: 0,
                    a: 255
                };
                4
            ],
            2,
            2,
        ));
        assert!(data.has_alpha());

        let data = PixelData::Gray8(ImgVec::new(vec![Gray::new(0u8); 4], 2, 2));
        assert!(!data.has_alpha());

        let data = PixelData::GrayF32(ImgVec::new(vec![Gray::new(0.0f32); 4], 2, 2));
        assert!(!data.has_alpha());
        assert_eq!(data.width(), 2);
        assert_eq!(data.height(), 2);
    }

    #[test]
    fn gray_alpha8_has_alpha() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(128, 200); 4], 2, 2));
        assert!(data.has_alpha());
        assert_eq!(data.width(), 2);
        assert_eq!(data.height(), 2);
    }

    #[test]
    fn to_bytes_rgb8() {
        let data = PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 1, g: 2, b: 3 }; 1], 1, 1));
        assert_eq!(data.to_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn to_bytes_gray8() {
        let data = PixelData::Gray8(ImgVec::new(vec![Gray::new(42u8); 2], 2, 1));
        assert_eq!(data.to_bytes(), vec![42, 42]);
    }

    #[test]
    fn to_bytes_rgba8() {
        let data = PixelData::Rgba8(ImgVec::new(
            vec![
                Rgba {
                    r: 1,
                    g: 2,
                    b: 3,
                    a: 4
                };
                1
            ],
            1,
            1,
        ));
        assert_eq!(data.to_bytes(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn gray_alpha_debug() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(0u8, 0); 6], 3, 2));
        assert_eq!(alloc::format!("{:?}", data), "PixelData::GrayAlpha8(3x2)");
    }

    #[test]
    fn debug_format() {
        let data = PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 0u8, g: 0, b: 0 }; 6], 3, 2));
        assert_eq!(alloc::format!("{:?}", data), "PixelData::Rgb8(3x2)");

        let data = PixelData::GrayF32(ImgVec::new(vec![Gray::new(0.0f32); 6], 3, 2));
        assert_eq!(alloc::format!("{:?}", data), "PixelData::GrayF32(3x2)");
    }

    #[test]
    fn as_pixel_slice_rgb8() {
        let data = PixelData::Rgb8(ImgVec::new(
            vec![
                Rgb {
                    r: 10,
                    g: 20,
                    b: 30,
                },
                Rgb {
                    r: 40,
                    g: 50,
                    b: 60,
                },
            ],
            2,
            1,
        ));
        let slice = data.as_pixel_slice().unwrap();
        assert_eq!(slice.width(), 2);
        assert_eq!(slice.rows(), 1);
        assert_eq!(slice.descriptor(), crate::buffer::PixelDescriptor::RGB8);
        assert_eq!(slice.row(0), &[10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn as_pixel_slice_gray_alpha_returns_none() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(0u8, 0)], 1, 1));
        assert!(data.as_pixel_slice().is_none());

        let data = PixelData::GrayAlpha16(ImgVec::new(vec![GrayAlpha::new(0u16, 0)], 1, 1));
        assert!(data.as_pixel_slice().is_none());

        let data = PixelData::GrayAlphaF32(ImgVec::new(vec![GrayAlpha::new(0.0f32, 0.0)], 1, 1));
        assert!(data.as_pixel_slice().is_none());
    }

    #[test]
    fn as_pixel_slice_all_non_gray_alpha() {
        // Verify all non-GrayAlpha variants return Some.
        let cases: Vec<PixelData> = vec![
            PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 0, g: 0, b: 0 }], 1, 1)),
            PixelData::Rgba8(ImgVec::new(
                vec![Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                }],
                1,
                1,
            )),
            PixelData::Rgb16(ImgVec::new(
                vec![Rgb {
                    r: 0u16,
                    g: 0,
                    b: 0,
                }],
                1,
                1,
            )),
            PixelData::Rgba16(ImgVec::new(
                vec![Rgba {
                    r: 0u16,
                    g: 0,
                    b: 0,
                    a: 0,
                }],
                1,
                1,
            )),
            PixelData::RgbF32(ImgVec::new(
                vec![Rgb {
                    r: 0.0f32,
                    g: 0.0,
                    b: 0.0,
                }],
                1,
                1,
            )),
            PixelData::RgbaF32(ImgVec::new(
                vec![Rgba {
                    r: 0.0f32,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }],
                1,
                1,
            )),
            PixelData::Gray8(ImgVec::new(vec![Gray::new(0u8)], 1, 1)),
            PixelData::Gray16(ImgVec::new(vec![Gray::new(0u16)], 1, 1)),
            PixelData::GrayF32(ImgVec::new(vec![Gray::new(0.0f32)], 1, 1)),
            PixelData::Bgra8(ImgVec::new(
                vec![BGRA {
                    b: 0,
                    g: 0,
                    r: 0,
                    a: 0,
                }],
                1,
                1,
            )),
        ];
        for data in cases {
            assert!(
                data.as_pixel_slice().is_some(),
                "as_pixel_slice returned None for {:?}",
                data
            );
        }
    }
}
