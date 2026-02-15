//! Image format detection and metadata.

/// Supported image formats.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    WebP,
    Gif,
    Png,
    Avif,
    Jxl,
    Pnm,
}

impl ImageFormat {
    /// Detect format from magic bytes. Returns `None` if unrecognized.
    pub fn detect(data: &[u8]) -> Option<Self> {
        // JPEG: FF D8 FF
        if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Some(ImageFormat::Jpeg);
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.len() >= 8 && data[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Some(ImageFormat::Png);
        }

        // GIF: "GIF87a" or "GIF89a"
        if data.len() >= 6
            && data[..3] == *b"GIF"
            && data[3] == b'8'
            && (data[4] == b'7' || data[4] == b'9')
            && data[5] == b'a'
        {
            return Some(ImageFormat::Gif);
        }

        // WebP: "RIFF....WEBP"
        if data.len() >= 12 && data[..4] == *b"RIFF" && data[8..12] == *b"WEBP" {
            return Some(ImageFormat::WebP);
        }

        // AVIF: ftyp box with avif/avis brand
        if data.len() >= 12 && &data[4..8] == b"ftyp" {
            let brand = &data[8..12];
            if brand == b"avif" || brand == b"avis" {
                return Some(ImageFormat::Avif);
            }
        }

        // JPEG XL codestream: FF 0A
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0x0A {
            return Some(ImageFormat::Jxl);
        }

        // JPEG XL container: 00 00 00 0C 4A 58 4C 20 0D 0A 87 0A
        if data.len() >= 12
            && data[..4] == [0x00, 0x00, 0x00, 0x0C]
            && data[4..8] == [b'J', b'X', b'L', b' ']
            && data[8..12] == [0x0D, 0x0A, 0x87, 0x0A]
        {
            return Some(ImageFormat::Jxl);
        }

        // PNM family: P1-P7, Pf (grayscale PFM), PF (color PFM)
        if data.len() >= 2 && data[0] == b'P' {
            match data[1] {
                b'1'..=b'7' | b'F' | b'f' => return Some(ImageFormat::Pnm),
                _ => {}
            }
        }

        None
    }

    /// Detect format from file extension (case-insensitive).
    pub fn from_extension(ext: &str) -> Option<Self> {
        // Manual case-insensitive comparison without std.
        let mut buf = [0u8; 8];
        let ext_bytes = ext.as_bytes();
        if ext_bytes.len() > buf.len() {
            return None;
        }
        for (i, &b) in ext_bytes.iter().enumerate() {
            buf[i] = b.to_ascii_lowercase();
        }
        let lower = &buf[..ext_bytes.len()];

        match lower {
            b"jpg" | b"jpeg" | b"jpe" | b"jfif" => Some(ImageFormat::Jpeg),
            b"webp" => Some(ImageFormat::WebP),
            b"gif" => Some(ImageFormat::Gif),
            b"png" => Some(ImageFormat::Png),
            b"avif" => Some(ImageFormat::Avif),
            b"jxl" => Some(ImageFormat::Jxl),
            b"pnm" | b"ppm" | b"pgm" | b"pbm" | b"pam" | b"pfm" => Some(ImageFormat::Pnm),
            _ => None,
        }
    }

    /// MIME type string.
    pub fn mime_type(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Png => "image/png",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Jxl => "image/jxl",
            ImageFormat::Pnm => "image/x-portable-anymap",
        }
    }

    /// Common file extensions.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            ImageFormat::WebP => &["webp"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::Png => &["png"],
            ImageFormat::Avif => &["avif"],
            ImageFormat::Jxl => &["jxl"],
            ImageFormat::Pnm => &["pnm", "ppm", "pgm", "pbm", "pam", "pfm"],
        }
    }

    /// Whether this format supports lossy encoding.
    pub fn supports_lossy(self) -> bool {
        matches!(
            self,
            ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Avif | ImageFormat::Jxl
        )
    }

    /// Whether this format supports lossless encoding.
    pub fn supports_lossless(self) -> bool {
        matches!(
            self,
            ImageFormat::WebP
                | ImageFormat::Gif
                | ImageFormat::Png
                | ImageFormat::Avif
                | ImageFormat::Jxl
                | ImageFormat::Pnm
        )
    }

    /// Whether this format supports animation.
    pub fn supports_animation(self) -> bool {
        matches!(
            self,
            ImageFormat::WebP | ImageFormat::Gif | ImageFormat::Jxl
        )
    }

    /// Recommended bytes to fetch for probing any format.
    ///
    /// 4096 bytes is enough for all formats including JPEG (which may have
    /// large EXIF/APP segments before the SOF marker).
    pub const RECOMMENDED_PROBE_BYTES: usize = 4096;

    /// Minimum bytes needed for reliable dimension probing of this format.
    ///
    /// With fewer bytes, format detection may succeed but dimensions may be
    /// missing from the probe result.
    pub fn min_probe_bytes(self) -> usize {
        match self {
            ImageFormat::Png => 33,    // 8 sig + 25 IHDR
            ImageFormat::Gif => 13,    // 6 header + 7 LSD
            ImageFormat::WebP => 30,   // RIFF(12) + chunk header + VP8X dims
            ImageFormat::Jpeg => 2048, // SOF can follow large EXIF/APP segments
            ImageFormat::Avif => 512,  // ISOBMFF box traversal (ftyp + meta)
            ImageFormat::Jxl => 256,   // codestream header or container + jxlc
            ImageFormat::Pnm => 20,    // magic + ASCII dimensions
        }
    }

    /// Whether this format supports alpha channel.
    pub fn supports_alpha(self) -> bool {
        !matches!(self, ImageFormat::Jpeg)
    }
}

impl core::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::WebP => "WebP",
            ImageFormat::Gif => "GIF",
            ImageFormat::Png => "PNG",
            ImageFormat::Avif => "AVIF",
            ImageFormat::Jxl => "JPEG XL",
            ImageFormat::Pnm => "PNM",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_jpeg() {
        assert_eq!(
            ImageFormat::detect(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn detect_png() {
        assert_eq!(
            ImageFormat::detect(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            Some(ImageFormat::Png)
        );
    }

    #[test]
    fn detect_gif() {
        assert_eq!(
            ImageFormat::detect(b"GIF89a\x00\x00"),
            Some(ImageFormat::Gif)
        );
    }

    #[test]
    fn detect_webp() {
        assert_eq!(
            ImageFormat::detect(b"RIFF\x00\x00\x00\x00WEBP"),
            Some(ImageFormat::WebP)
        );
    }

    #[test]
    fn detect_avif() {
        assert_eq!(
            ImageFormat::detect(b"\x00\x00\x00\x18ftypavif"),
            Some(ImageFormat::Avif)
        );
    }

    #[test]
    fn detect_jxl_codestream() {
        assert_eq!(ImageFormat::detect(&[0xFF, 0x0A]), Some(ImageFormat::Jxl));
    }

    #[test]
    fn detect_jxl_container() {
        assert_eq!(
            ImageFormat::detect(&[
                0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A
            ]),
            Some(ImageFormat::Jxl)
        );
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(ImageFormat::detect(b"nope"), None);
        assert_eq!(ImageFormat::detect(&[]), None);
    }

    #[test]
    fn from_extension_case_insensitive() {
        assert_eq!(ImageFormat::from_extension("JPG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("WebP"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("unknown"), None);
    }

    #[test]
    fn mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Jxl.mime_type(), "image/jxl");
    }

    #[test]
    fn probe_constants() {
        assert_eq!(ImageFormat::RECOMMENDED_PROBE_BYTES, 4096);
        assert!(ImageFormat::Jpeg.min_probe_bytes() > ImageFormat::Png.min_probe_bytes());
    }

    #[test]
    fn display_format() {
        assert_eq!(alloc::format!("{}", ImageFormat::Jpeg), "JPEG");
        assert_eq!(alloc::format!("{}", ImageFormat::WebP), "WebP");
        assert_eq!(alloc::format!("{}", ImageFormat::Gif), "GIF");
        assert_eq!(alloc::format!("{}", ImageFormat::Png), "PNG");
        assert_eq!(alloc::format!("{}", ImageFormat::Avif), "AVIF");
        assert_eq!(alloc::format!("{}", ImageFormat::Jxl), "JPEG XL");
    }

    #[test]
    fn from_extension_all_variants() {
        assert_eq!(ImageFormat::from_extension("jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("jpe"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("jfif"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("JPEG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("webp"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("gif"), Some(ImageFormat::Gif));
        assert_eq!(ImageFormat::from_extension("png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_extension("avif"), Some(ImageFormat::Avif));
        assert_eq!(ImageFormat::from_extension("jxl"), Some(ImageFormat::Jxl));
    }

    #[test]
    fn from_extension_edge_cases() {
        assert_eq!(ImageFormat::from_extension(""), None);
        assert_eq!(ImageFormat::from_extension("bmp"), None);
        assert_eq!(ImageFormat::from_extension("tiff"), None);
        // Too long for buffer
        assert_eq!(ImageFormat::from_extension("very_long_extension"), None);
    }

    #[test]
    fn capabilities() {
        assert!(ImageFormat::Jpeg.supports_lossy());
        assert!(!ImageFormat::Jpeg.supports_lossless());
        assert!(!ImageFormat::Jpeg.supports_animation());
        assert!(!ImageFormat::Jpeg.supports_alpha());

        assert!(ImageFormat::Png.supports_lossless());
        assert!(!ImageFormat::Png.supports_lossy());
        assert!(ImageFormat::Png.supports_alpha());
        assert!(!ImageFormat::Png.supports_animation());

        assert!(ImageFormat::WebP.supports_lossy());
        assert!(ImageFormat::WebP.supports_lossless());
        assert!(ImageFormat::WebP.supports_animation());
        assert!(ImageFormat::WebP.supports_alpha());

        assert!(ImageFormat::Gif.supports_animation());
        assert!(ImageFormat::Gif.supports_lossless());
        assert!(ImageFormat::Gif.supports_alpha());

        assert!(ImageFormat::Jxl.supports_lossy());
        assert!(ImageFormat::Jxl.supports_lossless());
        assert!(ImageFormat::Jxl.supports_animation());
    }

    #[test]
    fn extensions() {
        assert!(ImageFormat::Jpeg.extensions().contains(&"jpg"));
        assert!(ImageFormat::Jpeg.extensions().contains(&"jpeg"));
        assert_eq!(ImageFormat::Png.extensions(), &["png"]);
    }

    #[test]
    fn detect_pnm_p5() {
        assert_eq!(
            ImageFormat::detect(b"P5\n3 2\n255\n"),
            Some(ImageFormat::Pnm)
        );
    }

    #[test]
    fn detect_pnm_p6() {
        assert_eq!(
            ImageFormat::detect(b"P6\n3 2\n255\n"),
            Some(ImageFormat::Pnm)
        );
    }

    #[test]
    fn detect_pnm_p7() {
        assert_eq!(
            ImageFormat::detect(b"P7\nWIDTH 2\n"),
            Some(ImageFormat::Pnm)
        );
    }

    #[test]
    fn detect_pnm_pfm_color() {
        assert_eq!(ImageFormat::detect(b"PF\n3 2\n"), Some(ImageFormat::Pnm));
    }

    #[test]
    fn detect_pnm_pfm_gray() {
        assert_eq!(ImageFormat::detect(b"Pf\n3 2\n"), Some(ImageFormat::Pnm));
    }

    #[test]
    fn detect_pnm_p1_ascii() {
        assert_eq!(ImageFormat::detect(b"P1\n3 2\n"), Some(ImageFormat::Pnm));
    }

    #[test]
    fn from_extension_pnm_variants() {
        assert_eq!(ImageFormat::from_extension("pnm"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("ppm"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("pgm"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("pbm"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("pam"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("pfm"), Some(ImageFormat::Pnm));
        assert_eq!(ImageFormat::from_extension("PNM"), Some(ImageFormat::Pnm));
    }

    #[test]
    fn pnm_capabilities() {
        assert!(!ImageFormat::Pnm.supports_lossy());
        assert!(ImageFormat::Pnm.supports_lossless());
        assert!(!ImageFormat::Pnm.supports_animation());
        assert!(ImageFormat::Pnm.supports_alpha());
    }

    #[test]
    fn pnm_mime_type() {
        assert_eq!(ImageFormat::Pnm.mime_type(), "image/x-portable-anymap");
    }

    #[test]
    fn pnm_extensions() {
        let exts = ImageFormat::Pnm.extensions();
        assert!(exts.contains(&"pnm"));
        assert!(exts.contains(&"ppm"));
        assert!(exts.contains(&"pgm"));
        assert!(exts.contains(&"pbm"));
        assert!(exts.contains(&"pam"));
        assert!(exts.contains(&"pfm"));
    }

    #[test]
    fn pnm_display() {
        assert_eq!(alloc::format!("{}", ImageFormat::Pnm), "PNM");
    }

    #[test]
    fn pnm_min_probe_bytes() {
        assert_eq!(ImageFormat::Pnm.min_probe_bytes(), 20);
    }
}
