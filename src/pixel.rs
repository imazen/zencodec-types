//! Typed pixel buffer definitions.
//!
//! Uses `imgref::ImgVec` for 2D pixel data with typed pixels from the `rgb` crate.

use alloc::vec::Vec;
use imgref::ImgVec;
use rgb::alt::BGRA;
use rgb::{Gray, Rgb, Rgba};

/// Decoded pixel data in a typed buffer.
///
/// The variant determines both the pixel format and precision.
/// Width and height are embedded in the `ImgVec`.
#[non_exhaustive]
pub enum PixelData {
    Rgb8(ImgVec<Rgb<u8>>),
    Rgba8(ImgVec<Rgba<u8>>),
    Rgb16(ImgVec<Rgb<u16>>),
    Rgba16(ImgVec<Rgba<u16>>),
    RgbF32(ImgVec<Rgb<f32>>),
    RgbaF32(ImgVec<Rgba<f32>>),
    Gray8(ImgVec<Gray<u8>>),
    Gray16(ImgVec<Gray<u16>>),
    /// 8-bit BGRA (blue, green, red, alpha byte order).
    ///
    /// Native byte order for Windows/DirectX surfaces. Codecs that support
    /// BGRA natively (e.g. zenjpeg, zenwebp) can consume this without
    /// an intermediate channel swizzle.
    Bgra8(ImgVec<BGRA<u8>>),
}

impl PixelData {
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
            PixelData::Bgra8(img) => img.width() as u32,
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
            PixelData::Bgra8(img) => img.height() as u32,
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
        )
    }

    /// Convert to RGB8 by reference, allocating a new buffer.
    ///
    /// Gray8 is expanded to RGB with R=G=B=gray.
    /// RGBA variants discard alpha.
    /// Higher-precision formats are clamped/truncated to 8-bit.
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
                        let v = (p.value() >> 8) as u8;
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
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
                    })
                    .collect();
                ImgVec::new(rgb, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgb: Vec<Rgb<u8>> = buf
                    .iter()
                    .map(|p| Rgb {
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
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
        }
    }

    /// Convert to RGBA8 by reference, allocating a new buffer.
    ///
    /// Gray8 is expanded to RGBA with R=G=B=gray, A=255.
    /// RGB variants get A=255 added.
    /// Higher-precision formats are clamped/truncated to 8-bit.
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
                        let v = (p.value() >> 8) as u8;
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
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
                        a: (p.a >> 8) as u8,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
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
    pub fn as_bytes(&self) -> Vec<u8> {
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
            PixelData::Bgra8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
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
            PixelData::Bgra8(_) => "Bgra8",
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

        let img = ImgVec::new(
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
        );
        let data = PixelData::Rgba8(img);
        assert!(data.has_alpha());
    }

    #[test]
    fn rgb8_to_rgba8() {
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
        let data = PixelData::Rgb8(img);
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
        let img = ImgVec::new(pixels, 3, 2);
        let data = PixelData::Rgb8(img);
        let result = data.into_rgb8();
        // Same allocation — no clone happened.
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
        let img = ImgVec::new(pixels, 3, 2);
        let data = PixelData::Rgba8(img);
        let result = data.into_rgba8();
        assert_eq!(result.buf().as_ptr(), ptr);
    }

    #[test]
    fn gray8_to_rgb8() {
        let img = ImgVec::new(vec![Gray::new(128u8); 4], 2, 2);
        let data = PixelData::Gray8(img);
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (128, 128, 128));
    }

    #[test]
    fn f32_clamped() {
        let img = ImgVec::new(
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
        );
        let data = PixelData::RgbF32(img);
        let rgb = data.to_rgb8();
        let px = &rgb.buf()[0];
        assert_eq!((px.r, px.g, px.b), (0, 127, 255));
    }

    #[test]
    fn debug_format() {
        let img = ImgVec::new(vec![Rgb { r: 0u8, g: 0, b: 0 }; 6], 3, 2);
        let data = PixelData::Rgb8(img);
        let s = alloc::format!("{:?}", data);
        assert_eq!(s, "PixelData::Rgb8(3x2)");
    }
}
