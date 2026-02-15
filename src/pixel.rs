//! Typed pixel buffer definitions.
//!
//! Uses `imgref::ImgVec` for 2D pixel data with typed pixels from the `rgb` crate.

use alloc::vec::Vec;
use imgref::ImgVec;
use rgb::alt::BGRA;
use rgb::{Gray, Rgb, Rgba};

/// Grayscale pixel with alpha channel.
///
/// A simple two-component pixel type. Not from the `rgb` crate — we own this
/// type to avoid API instability in `rgb::alt::GrayAlpha`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
/// # Transfer function conventions
///
/// - **u8 / u16 variants**: Values are in the image's native transfer function,
///   typically sRGB gamma. The actual transfer function is indicated by the
///   CICP transfer characteristics in [`ImageInfo`](crate::ImageInfo). u16 variants
///   use the full 0–65535 range regardless of source bit depth (e.g. 10-bit
///   AVIF values are scaled up, not left in 0–1023).
///
/// - **f32 variants**: Values are in **linear light** (gamma removed, scene-referred).
///   Range is [0.0, 1.0] for SDR content. HDR content (PQ/HLG) may exceed 1.0
///   and the CICP transfer characteristics indicate the original encoding.
///
/// Codecs perform the linearization when producing f32 output and the gamma
/// encoding when producing u8/u16 output. If you need to convert between
/// gamma-encoded and linear yourself, check the CICP transfer characteristics
/// in the image metadata.
#[non_exhaustive]
pub enum PixelData {
    /// 8-bit RGB in the image's native transfer function (typically sRGB).
    Rgb8(ImgVec<Rgb<u8>>),
    /// 8-bit RGBA in the image's native transfer function (typically sRGB).
    Rgba8(ImgVec<Rgba<u8>>),
    /// 16-bit RGB in the image's native transfer function (typically sRGB).
    ///
    /// Full 0–65535 range regardless of source bit depth.
    Rgb16(ImgVec<Rgb<u16>>),
    /// 16-bit RGBA in the image's native transfer function (typically sRGB).
    ///
    /// Full 0–65535 range regardless of source bit depth.
    Rgba16(ImgVec<Rgba<u16>>),
    /// Linear-light RGB f32. See [transfer function conventions](PixelData#transfer-function-conventions).
    RgbF32(ImgVec<Rgb<f32>>),
    /// Linear-light RGBA f32. See [transfer function conventions](PixelData#transfer-function-conventions).
    RgbaF32(ImgVec<Rgba<f32>>),
    /// 8-bit grayscale in the image's native transfer function.
    Gray8(ImgVec<Gray<u8>>),
    /// 16-bit grayscale in the image's native transfer function.
    ///
    /// Full 0–65535 range regardless of source bit depth.
    Gray16(ImgVec<Gray<u16>>),
    /// Linear-light grayscale f32. See [transfer function conventions](PixelData#transfer-function-conventions).
    GrayF32(ImgVec<Gray<f32>>),
    /// 8-bit BGRA (blue, green, red, alpha byte order).
    ///
    /// Native byte order for Windows/DirectX surfaces. Codecs that support
    /// BGRA natively (e.g. zenjpeg, zenwebp) can consume this without
    /// an intermediate channel swizzle.
    Bgra8(ImgVec<BGRA<u8>>),
    /// 8-bit grayscale with alpha channel.
    GrayAlpha8(ImgVec<GrayAlpha<u8>>),
    /// 16-bit grayscale with alpha channel.
    ///
    /// Full 0–65535 range regardless of source bit depth.
    GrayAlpha16(ImgVec<GrayAlpha<u16>>),
    /// Linear-light grayscale + alpha f32.
    GrayAlphaF32(ImgVec<GrayAlpha<f32>>),
}

impl PixelData {
    /// Pixel format descriptor for this variant.
    ///
    /// Returns `Srgb` transfer for u8/u16 variants and `Linear` for f32
    /// variants. Alpha variants use [`AlphaMode::Straight`](crate::AlphaMode::Straight).
    /// Callers with CICP metadata can override the transfer function.
    pub fn descriptor(&self) -> crate::buffer::PixelDescriptor {
        use crate::buffer::PixelDescriptor;
        match self {
            PixelData::Rgb8(_) => PixelDescriptor::RGB8_SRGB,
            PixelData::Rgba8(_) => PixelDescriptor::RGBA8_SRGB,
            PixelData::Rgb16(_) => PixelDescriptor::RGB16_SRGB,
            PixelData::Rgba16(_) => PixelDescriptor::RGBA16_SRGB,
            PixelData::RgbF32(_) => PixelDescriptor::RGBF32_LINEAR,
            PixelData::RgbaF32(_) => PixelDescriptor::RGBAF32_LINEAR,
            PixelData::Gray8(_) => PixelDescriptor::GRAY8_SRGB,
            PixelData::Gray16(_) => PixelDescriptor::GRAY16_SRGB,
            PixelData::GrayF32(_) => PixelDescriptor::GRAYF32_LINEAR,
            PixelData::Bgra8(_) => PixelDescriptor::BGRA8_SRGB,
            PixelData::GrayAlpha8(_) => PixelDescriptor::GRAYA8_SRGB,
            PixelData::GrayAlpha16(_) => PixelDescriptor::GRAYA16_SRGB,
            PixelData::GrayAlphaF32(_) => PixelDescriptor::GRAYAF32_LINEAR,
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

    /// Convert to RGB8 by reference, allocating a new buffer.
    ///
    /// Assumes sRGB-encoded values. 16-bit values are downscaled with
    /// proper rounding. Float values are clamped to [0.0, 1.0].
    /// Gray is expanded to RGB with R=G=B. RGBA variants discard alpha.
    pub fn to_rgb8(&self) -> ImgVec<Rgb<u8>> {
        match self {
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value();
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = u16_to_u8(p.value());
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = (p.value().clamp(0.0, 1.0) * 255.0) as u8;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: u16_to_u8(p.r),
                        g: u16_to_u8(p.g),
                        b: u16_to_u8(p.b),
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: u16_to_u8(p.r),
                        g: u16_to_u8(p.g),
                        b: u16_to_u8(p.b),
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::RgbF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::RgbaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.v;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = u16_to_u8(p.v);
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = (p.v.clamp(0.0, 1.0) * 255.0) as u8;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
        }
    }

    /// Convert to RGBA8 by reference, allocating a new buffer.
    ///
    /// Assumes sRGB-encoded values. 16-bit values are downscaled with
    /// proper rounding. Float values are clamped to [0.0, 1.0].
    /// Gray is expanded with R=G=B=gray, A=255. RGB gets A=255 added.
    pub fn to_rgba8(&self) -> ImgVec<Rgba<u8>> {
        match self {
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value();
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 255,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = u16_to_u8(p.value());
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 255,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| {
                        let v = (p.value().clamp(0.0, 1.0) * 255.0) as u8;
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 255,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: u16_to_u8(p.r),
                        g: u16_to_u8(p.g),
                        b: u16_to_u8(p.b),
                        a: u16_to_u8(p.a),
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: u16_to_u8(p.r),
                        g: u16_to_u8(p.g),
                        b: u16_to_u8(p.b),
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::RgbF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::RgbaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                        a: (p.a.clamp(0.0, 1.0) * 255.0) as u8,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                        a: p.a,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.v,
                        g: p.v,
                        b: p.v,
                        a: p.a,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: u16_to_u8(p.v),
                        g: u16_to_u8(p.v),
                        b: u16_to_u8(p.v),
                        a: u16_to_u8(p.a),
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.v.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.v.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.v.clamp(0.0, 1.0) * 255.0) as u8,
                        a: (p.a.clamp(0.0, 1.0) * 255.0) as u8,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
        }
    }

    /// Convert to Gray8 by reference, allocating a new buffer.
    ///
    /// Assumes sRGB-encoded values. 16-bit values are downscaled with
    /// proper rounding. RGB variants use BT.601 luminance
    /// (0.299*R + 0.587*G + 0.114*B). RGBA/BGRA ignore alpha.
    pub fn to_gray8(&self) -> ImgVec<Gray<u8>> {
        match self {
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new(u16_to_u8(p.value())))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new(rgb_to_luma(p.r, p.g, p.b)))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new(rgb_to_luma(p.r, p.g, p.b)))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new(rgb_to_luma(p.r, p.g, p.b)))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new((p.value().clamp(0.0, 1.0) * 255.0) as u8))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf.iter().map(|p| Gray::new(p.v)).collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf.iter().map(|p| Gray::new(u16_to_u8(p.v))).collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new((p.v.clamp(0.0, 1.0) * 255.0) as u8))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            other => {
                // Fall back through Rgb8 for all other formats.
                let rgb = other.to_rgb8();
                let (buf, w, h) = rgb.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<u8>> = buf
                    .iter()
                    .map(|p| Gray::new(rgb_to_luma(p.r, p.g, p.b)))
                    .collect();
                ImgVec::new(gray, w, h)
            }
        }
    }

    /// Convert to GrayF32, consuming self.
    ///
    /// Avoids a clone when the data is already GrayF32.
    /// Converts through Gray8 for non-float, non-gray formats.
    pub fn into_gray_f32(self) -> ImgVec<Gray<f32>> {
        match self {
            PixelData::GrayF32(img) => img,
            other => other.to_gray_f32(),
        }
    }

    /// Convert to RgbF32 by reference, allocating a new buffer.
    ///
    /// Values are in [0.0, 1.0]. Conversion from integer formats divides
    /// by the type's maximum value.
    pub fn to_rgb_f32(&self) -> ImgVec<Rgb<f32>> {
        match self {
            PixelData::RgbF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::RgbaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value();
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value() as f32 / 255.0;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value() as f32 / 65535.0;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r as f32 / 65535.0,
                        g: p.g as f32 / 65535.0,
                        b: p.b as f32 / 65535.0,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r as f32 / 65535.0,
                        g: p.g as f32 / 65535.0,
                        b: p.b as f32 / 65535.0,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.v as f32 / 255.0;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.v as f32 / 65535.0;
                        Rgb { r: v, g: v, b: v }
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<f32>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: p.v,
                        g: p.v,
                        b: p.v,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
        }
    }

    /// Convert to RgbF32, consuming self.
    ///
    /// Avoids a clone when the data is already RgbF32.
    pub fn into_rgb_f32(self) -> ImgVec<Rgb<f32>> {
        match self {
            PixelData::RgbF32(img) => img,
            other => other.to_rgb_f32(),
        }
    }

    /// Convert to RgbaF32 by reference, allocating a new buffer.
    ///
    /// Values are in [0.0, 1.0]. Non-alpha formats get alpha = 1.0.
    pub fn to_rgba_f32(&self) -> ImgVec<Rgba<f32>> {
        match self {
            PixelData::RgbaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::RgbF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                        a: 1.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value();
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 1.0,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                        a: p.a as f32 / 255.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                        a: 1.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value() as f32 / 255.0;
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 1.0,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.value() as f32 / 65535.0;
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: 1.0,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r as f32 / 65535.0,
                        g: p.g as f32 / 65535.0,
                        b: p.b as f32 / 65535.0,
                        a: 1.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r as f32 / 65535.0,
                        g: p.g as f32 / 65535.0,
                        b: p.b as f32 / 65535.0,
                        a: p.a as f32 / 65535.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r as f32 / 255.0,
                        g: p.g as f32 / 255.0,
                        b: p.b as f32 / 255.0,
                        a: p.a as f32 / 255.0,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.v as f32 / 255.0;
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: p.a as f32 / 255.0,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| {
                        let v = p.v as f32 / 65535.0;
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: p.a as f32 / 65535.0,
                        }
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<f32>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.v,
                        g: p.v,
                        b: p.v,
                        a: p.a,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
        }
    }

    /// Convert to RgbaF32, consuming self.
    ///
    /// Avoids a clone when the data is already RgbaF32.
    pub fn into_rgba_f32(self) -> ImgVec<Rgba<f32>> {
        match self {
            PixelData::RgbaF32(img) => img,
            other => other.to_rgba_f32(),
        }
    }

    /// Convert to GrayF32 by reference, allocating a new buffer.
    ///
    /// Values are in [0.0, 1.0]. Conversion from integer formats divides
    /// by the type's maximum value. This assumes sRGB-encoded values.
    pub fn to_gray_f32(&self) -> ImgVec<Gray<f32>> {
        match self {
            PixelData::GrayF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> = buf
                    .iter()
                    .map(|p| Gray::new(p.value() as f32 / 255.0))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::Gray16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> = buf
                    .iter()
                    .map(|p| Gray::new(p.value() as f32 / 65535.0))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlpha8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> =
                    buf.iter().map(|p| Gray::new(p.v as f32 / 255.0)).collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlpha16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> = buf
                    .iter()
                    .map(|p| Gray::new(p.v as f32 / 65535.0))
                    .collect();
                ImgVec::new(gray, w, h)
            }
            PixelData::GrayAlphaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> = buf.iter().map(|p| Gray::new(p.v)).collect();
                ImgVec::new(gray, w, h)
            }
            other => {
                // Convert through rgb8, then to gray float.
                let rgb = other.to_rgb8();
                let (buf, w, h) = rgb.as_ref().to_contiguous_buf();
                let gray: Vec<Gray<f32>> = buf
                    .iter()
                    .map(|p| Gray::new(rgb_to_luma(p.r, p.g, p.b) as f32 / 255.0))
                    .collect();
                ImgVec::new(gray, w, h)
            }
        }
    }

    /// Convert to Gray8, consuming self.
    ///
    /// Avoids a clone when the data is already Gray8.
    pub fn into_gray8(self) -> ImgVec<Gray<u8>> {
        match self {
            PixelData::Gray8(img) => img,
            other => other.to_gray8(),
        }
    }

    /// Convert to BGRA8 by reference, allocating a new buffer.
    ///
    /// Bgra8 is cloned. RGB/RGBA variants have channels reordered.
    /// Higher-precision formats are clamped/truncated to 8-bit.
    pub fn to_bgra8(&self) -> ImgVec<BGRA<u8>> {
        match self {
            PixelData::Bgra8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let bgra: Vec<BGRA<u8>> = buf
                    .iter()
                    .map(|p| BGRA {
                        b: p.b,
                        g: p.g,
                        r: p.r,
                        a: p.a,
                    })
                    .collect();
                ImgVec::new(bgra, w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let bgra: Vec<BGRA<u8>> = buf
                    .iter()
                    .map(|p| BGRA {
                        b: p.b,
                        g: p.g,
                        r: p.r,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(bgra, w, h)
            }
            other => {
                // Fall back through RGBA for all other formats.
                let rgba = other.to_rgba8();
                let (buf, w, h) = rgba.as_ref().to_contiguous_buf();
                let bgra: Vec<BGRA<u8>> = buf
                    .iter()
                    .map(|p| BGRA {
                        b: p.b,
                        g: p.g,
                        r: p.r,
                        a: p.a,
                    })
                    .collect();
                ImgVec::new(bgra, w, h)
            }
        }
    }

    /// Convert to BGRA8, consuming self.
    ///
    /// Avoids a clone when the data is already Bgra8.
    pub fn into_bgra8(self) -> ImgVec<BGRA<u8>> {
        match self {
            PixelData::Bgra8(img) => img,
            other => other.to_bgra8(),
        }
    }

    /// Convert to RGB8, consuming self.
    ///
    /// Avoids a clone when the data is already Rgb8.
    pub fn into_rgb8(self) -> ImgVec<Rgb<u8>> {
        match self {
            PixelData::Rgb8(img) => img,
            other => other.to_rgb8(),
        }
    }

    /// Convert to RGBA8, consuming self.
    ///
    /// Avoids a clone when the data is already Rgba8.
    pub fn into_rgba8(self) -> ImgVec<Rgba<u8>> {
        match self {
            PixelData::Rgba8(img) => img,
            other => other.to_rgba8(),
        }
    }

    /// Get the raw pixel data as a byte vector.
    ///
    /// This allocates a new `Vec<u8>` — use `to_rgb8()`/`to_rgba8()` for
    /// typed access without raw byte conversion.
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

/// BT.601 luminance from 8-bit RGB. Matches JPEG's grayscale conversion.
fn rgb_to_luma(r: u8, g: u8, b: u8) -> u8 {
    // Fixed-point: 0.299*256=77, 0.587*256=150, 0.114*256=29 (sum=256)
    ((77u32 * r as u32 + 150u32 * g as u32 + 29u32 * b as u32) >> 8) as u8
}

/// Convert 16-bit to 8-bit with proper rounding.
///
/// Uses `(v * 255 + 32768) >> 16` which maps 0→0 and 65535→255 exactly,
/// with correct rounding for intermediate values. Assumes sRGB-encoded data
/// (i.e. the 16-bit values are in the sRGB transfer curve, not linear light).
#[inline]
fn u16_to_u8(v: u16) -> u8 {
    ((v as u32 * 255 + 32768) >> 16) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // --- dimensions and alpha ---

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

    // --- RGB8 conversions ---

    #[test]
    fn rgb8_to_rgba8() {
        let data = PixelData::Rgb8(ImgVec::new(
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
        ));
        let rgba = data.to_rgba8();
        assert_eq!(rgba.width(), 2);
        assert_eq!(rgba.height(), 2);
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (10, 20, 30, 255));
    }

    #[test]
    fn into_rgb8_no_clone() {
        let pixels = vec![Rgb { r: 1u8, g: 2, b: 3 }; 6];
        let ptr = pixels.as_ptr();
        let data = PixelData::Rgb8(ImgVec::new(pixels, 3, 2));
        let result = data.into_rgb8();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    #[test]
    fn into_rgba8_no_clone() {
        let pixels = vec![
            Rgba {
                r: 1u8,
                g: 2,
                b: 3,
                a: 4
            };
            6
        ];
        let ptr = pixels.as_ptr();
        let data = PixelData::Rgba8(ImgVec::new(pixels, 3, 2));
        let result = data.into_rgba8();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    // --- Gray conversions ---

    #[test]
    fn gray8_to_rgb8() {
        let data = PixelData::Gray8(ImgVec::new(vec![Gray::new(128u8); 4], 2, 2));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (128, 128, 128));
    }

    #[test]
    fn gray8_to_rgba8() {
        let data = PixelData::Gray8(ImgVec::new(vec![Gray::new(200u8); 1], 1, 1));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (200, 200, 200, 255));
    }

    #[test]
    fn into_gray8_no_clone() {
        let pixels = vec![Gray::new(42u8); 6];
        let ptr = pixels.as_ptr();
        let data = PixelData::Gray8(ImgVec::new(pixels, 3, 2));
        let result = data.into_gray8();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    #[test]
    fn rgb8_to_gray8_luma() {
        // Pure red: BT.601 luma = 0.299 * 255 ≈ 76
        let data = PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 255, g: 0, b: 0 }; 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 76);

        // Pure green: BT.601 luma = 0.587 * 255 ≈ 149
        let data = PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 0, g: 255, b: 0 }; 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 149);

        // Pure blue: BT.601 luma = 0.114 * 255 ≈ 28
        let data = PixelData::Rgb8(ImgVec::new(vec![Rgb { r: 0, g: 0, b: 255 }; 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 28);
    }

    #[test]
    fn rgba8_to_gray8_ignores_alpha() {
        let data = PixelData::Rgba8(ImgVec::new(
            vec![
                Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 0
                };
                1
            ], // fully transparent red
            1,
            1,
        ));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 76); // alpha ignored
    }

    #[test]
    fn bgra8_to_gray8() {
        let data = PixelData::Bgra8(ImgVec::new(
            vec![
                BGRA {
                    b: 0,
                    g: 0,
                    r: 255,
                    a: 255
                };
                1
            ],
            1,
            1,
        ));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 76); // same as pure red
    }

    // --- 16-bit conversions with proper rounding ---

    #[test]
    fn u16_to_u8_boundary_values() {
        assert_eq!(u16_to_u8(0), 0);
        assert_eq!(u16_to_u8(65535), 255);
        // Midpoint: 32768 → round(32768/257) = round(127.5) = 128
        assert_eq!(u16_to_u8(32768), 128);
        // 257 is exactly 1/255th of 65535
        assert_eq!(u16_to_u8(257), 1);
        assert_eq!(u16_to_u8(514), 2);
    }

    #[test]
    fn gray16_to_rgb8() {
        let data = PixelData::Gray16(ImgVec::new(vec![Gray::new(65535u16); 1], 1, 1));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (255, 255, 255));

        let data = PixelData::Gray16(ImgVec::new(vec![Gray::new(0u16); 1], 1, 1));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (0, 0, 0));
    }

    #[test]
    fn gray16_to_gray8() {
        let data = PixelData::Gray16(ImgVec::new(vec![Gray::new(32768u16); 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 128);
    }

    #[test]
    fn rgb16_to_rgb8() {
        let data = PixelData::Rgb16(ImgVec::new(
            vec![
                Rgb {
                    r: 65535u16,
                    g: 32768,
                    b: 0
                };
                1
            ],
            1,
            1,
        ));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (255, 128, 0));
    }

    #[test]
    fn rgba16_to_rgba8() {
        let data = PixelData::Rgba16(ImgVec::new(
            vec![
                Rgba {
                    r: 65535u16,
                    g: 0,
                    b: 32768,
                    a: 65535,
                };
                1
            ],
            1,
            1,
        ));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (255, 0, 128, 255));
    }

    // --- Float conversions ---

    #[test]
    fn f32_clamped() {
        let data = PixelData::RgbF32(ImgVec::new(
            vec![
                Rgb {
                    r: -0.5f32,
                    g: 0.5,
                    b: 1.5
                };
                1
            ],
            1,
            1,
        ));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (0, 127, 255));
    }

    #[test]
    fn rgba_f32_to_rgba8() {
        let data = PixelData::RgbaF32(ImgVec::new(
            vec![
                Rgba {
                    r: 1.0f32,
                    g: 0.0,
                    b: 0.5,
                    a: 0.75
                };
                1
            ],
            1,
            1,
        ));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (255, 0, 127, 191));
    }

    #[test]
    fn gray_f32_to_rgb8() {
        let data = PixelData::GrayF32(ImgVec::new(vec![Gray::new(0.5f32); 1], 1, 1));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (127, 127, 127));
    }

    #[test]
    fn gray_f32_to_gray8() {
        let data = PixelData::GrayF32(ImgVec::new(vec![Gray::new(1.0f32); 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 255);

        let data = PixelData::GrayF32(ImgVec::new(vec![Gray::new(0.0f32); 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 0);
    }

    #[test]
    fn gray_f32_roundtrip() {
        let data = PixelData::Gray8(ImgVec::new(vec![Gray::new(128u8); 1], 1, 1));
        let f32_img = data.to_gray_f32();
        let v = f32_img.buf()[0].value();
        assert!((v - 128.0 / 255.0).abs() < 0.001);
    }

    #[test]
    fn into_gray_f32_no_clone() {
        let pixels = vec![Gray::new(0.5f32); 6];
        let ptr = pixels.as_ptr();
        let data = PixelData::GrayF32(ImgVec::new(pixels, 3, 2));
        let result = data.into_gray_f32();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    // --- BGRA conversions ---

    #[test]
    fn bgra8_to_rgb8() {
        let data = PixelData::Bgra8(ImgVec::new(
            vec![
                BGRA {
                    b: 30,
                    g: 20,
                    r: 10,
                    a: 255
                };
                1
            ],
            1,
            1,
        ));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (10, 20, 30));
    }

    #[test]
    fn bgra8_to_rgba8() {
        let data = PixelData::Bgra8(ImgVec::new(
            vec![
                BGRA {
                    b: 30,
                    g: 20,
                    r: 10,
                    a: 128
                };
                1
            ],
            1,
            1,
        ));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (10, 20, 30, 128));
    }

    #[test]
    fn rgba8_to_bgra8() {
        let data = PixelData::Rgba8(ImgVec::new(
            vec![
                Rgba {
                    r: 10,
                    g: 20,
                    b: 30,
                    a: 128
                };
                1
            ],
            1,
            1,
        ));
        let bgra = data.to_bgra8();
        let px = &bgra.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (10, 20, 30, 128));
    }

    #[test]
    fn rgb8_to_bgra8() {
        let data = PixelData::Rgb8(ImgVec::new(
            vec![
                Rgb {
                    r: 10,
                    g: 20,
                    b: 30
                };
                1
            ],
            1,
            1,
        ));
        let bgra = data.to_bgra8();
        let px = &bgra.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (10, 20, 30, 255));
    }

    #[test]
    fn into_bgra8_no_clone() {
        let pixels = vec![
            BGRA {
                b: 0,
                g: 0,
                r: 0,
                a: 0
            };
            6
        ];
        let ptr = pixels.as_ptr();
        let data = PixelData::Bgra8(ImgVec::new(pixels, 3, 2));
        let result = data.into_bgra8();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    #[test]
    fn gray16_to_bgra8_through_fallback() {
        let data = PixelData::Gray16(ImgVec::new(vec![Gray::new(65535u16); 1], 1, 1));
        let bgra = data.to_bgra8();
        let px = &bgra.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (255, 255, 255, 255));
    }

    // --- to_bytes ---

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

    // --- Debug ---

    // --- GrayAlpha conversions ---

    #[test]
    fn gray_alpha8_has_alpha() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(128, 200); 4], 2, 2));
        assert!(data.has_alpha());
        assert_eq!(data.width(), 2);
        assert_eq!(data.height(), 2);
    }

    #[test]
    fn gray_alpha8_to_rgba8() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(128, 200); 1], 1, 1));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (128, 128, 128, 200));
    }

    #[test]
    fn gray_alpha8_to_rgb8() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(200u8, 50); 1], 1, 1));
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (200, 200, 200));
    }

    #[test]
    fn gray_alpha8_to_gray8() {
        let data = PixelData::GrayAlpha8(ImgVec::new(vec![GrayAlpha::new(42u8, 255); 1], 1, 1));
        let gray = data.to_gray8();
        assert_eq!(gray.buf()[0].value(), 42);
    }

    #[test]
    fn gray_alpha16_to_rgba8() {
        let data =
            PixelData::GrayAlpha16(ImgVec::new(vec![GrayAlpha::new(65535u16, 32768); 1], 1, 1));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (255, 255, 255, 128));
    }

    #[test]
    fn gray_alpha_f32_to_rgba8() {
        let data =
            PixelData::GrayAlphaF32(ImgVec::new(vec![GrayAlpha::new(0.5f32, 0.75); 1], 1, 1));
        let rgba = data.to_rgba8();
        let px = &rgba.buf()[0];
        assert_eq!((px.r, px.g, px.b, px.a), (127, 127, 127, 191));
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
}
