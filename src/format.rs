//! Image format detection and metadata.

/// Metadata for a format not known to zencodec-types.
///
/// Define as a `static` and reference via [`ImageFormat::Custom`].
/// Identity is based on `name` — two custom formats with the same name
/// are considered equal.
///
/// # Example
///
/// ```rust,ignore
/// use zc::{CustomImageFormat, ImageFormat};
///
/// fn detect_jpeg2000(data: &[u8]) -> bool {
///     data.len() >= 12 && data[..4] == [0x00, 0x00, 0x00, 0x0C]
///         && &data[4..8] == b"jP  "
/// }
///
/// static JPEG2000: CustomImageFormat = CustomImageFormat {
///     name: "jpeg2000",
///     display_name: "JPEG 2000",
///     preferred_extension: "jp2",
///     extensions: &["jp2", "j2k", "jpx"],
///     preferred_mime_type: "image/jp2",
///     mime_types: &["image/jp2", "image/jpx"],
///     supports_alpha: true,
///     supports_animation: false,
///     supports_lossless: true,
///     supports_lossy: true,
///     magic_bytes_needed: 12,
///     detect: detect_jpeg2000,
/// };
///
/// let fmt = ImageFormat::Custom(&JPEG2000);
/// assert_eq!(fmt.mime_type(), "image/jp2");
/// assert_eq!(fmt.extension(), "jp2");
/// ```
pub struct CustomImageFormat {
    /// Unique lowercase format identifier (e.g. `"jpeg2000"`, `"dds"`).
    ///
    /// Used for equality comparison and hashing. Must be unique across
    /// all custom formats in use.
    pub name: &'static str,

    /// Human-readable format name for display (e.g. `"JPEG 2000"`, `"DDS"`).
    pub display_name: &'static str,

    /// Primary file extension without dot (e.g. `"jp2"`).
    pub preferred_extension: &'static str,

    /// All recognized file extensions (must include `preferred_extension`).
    pub extensions: &'static [&'static str],

    /// Primary MIME type (e.g. `"image/jp2"`).
    pub preferred_mime_type: &'static str,

    /// All recognized MIME types (must include `preferred_mime_type`).
    pub mime_types: &'static [&'static str],

    /// Whether this format supports alpha channel.
    pub supports_alpha: bool,

    /// Whether this format supports animation.
    pub supports_animation: bool,

    /// Whether this format supports lossless encoding.
    pub supports_lossless: bool,

    /// Whether this format supports lossy encoding.
    pub supports_lossy: bool,

    /// Minimum bytes needed for reliable magic byte detection.
    pub magic_bytes_needed: usize,

    /// Magic byte detection function.
    ///
    /// Returns `true` if the data appears to be this format.
    /// The input will have at least `magic_bytes_needed` bytes.
    pub detect: fn(&[u8]) -> bool,
}

impl PartialEq for CustomImageFormat {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CustomImageFormat {}

impl core::hash::Hash for CustomImageFormat {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl core::fmt::Debug for CustomImageFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CustomImageFormat")
            .field("name", &self.name)
            .field("display_name", &self.display_name)
            .finish()
    }
}

/// Supported image formats.
///
/// Includes well-known formats as named variants and a [`Custom`](ImageFormat::Custom)
/// variant for formats defined by downstream crates.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Avif,
    Jxl,
    Heic,
    Bmp,
    Tiff,
    Ico,
    Pnm,
    Farbfeld,
    Qoi,
    Unknown,
    /// Format not known to zencodec-types.
    ///
    /// Define a [`CustomImageFormat`] as a `static` and reference it here.
    /// The custom format carries its own metadata (extensions, MIME types,
    /// detection function, capability flags).
    Custom(&'static CustomImageFormat),
}

impl ImageFormat {
    /// Detect format from magic bytes. Returns [`Unknown`](ImageFormat::Unknown) if unrecognized.
    ///
    /// Only detects built-in formats. For custom format detection, use
    /// [`CustomImageFormat::detect`] or a codec registry.
    pub fn from_magic(data: &[u8]) -> Self {
        Self::detect(data).unwrap_or(Self::Unknown)
    }

    /// Detect format from magic bytes. Returns `None` if unrecognized.
    ///
    /// Only detects built-in formats. For custom format detection, use
    /// [`CustomImageFormat::detect`] or a codec registry.
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

        // ISOBMFF ftyp box: AVIF and HEIC share the same container structure
        // ftyp layout: [4 bytes size][ftyp][4 bytes major_brand][4 bytes minor_version][compatible_brands...]
        if data.len() >= 12 && &data[4..8] == b"ftyp" {
            let major = &data[8..12];

            // AVIF: avif/avis major brand
            if major == b"avif" || major == b"avis" {
                return Some(ImageFormat::Avif);
            }

            // HEIC: heic/heix/hevc/hevx/heim/heis/hevm/hevs major brand
            const HEIC_BRANDS: &[&[u8; 4]] = &[
                b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"hevm", b"hevs",
            ];
            if HEIC_BRANDS.iter().any(|b| major == *b) {
                return Some(ImageFormat::Heic);
            }

            // mif1 is ambiguous — scan compatible brands to disambiguate
            if major == b"mif1" {
                // Read ftyp box size to bound the compatible brand scan
                let box_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
                let end = box_size.min(data.len());
                // Compatible brands start at offset 16 (after size + ftyp + major + minor_version)
                let mut offset = 16;
                let mut found_avif = false;
                let mut found_heic = false;
                while offset + 4 <= end {
                    let compat = &data[offset..offset + 4];
                    if compat == b"avif" || compat == b"avis" {
                        found_avif = true;
                    }
                    if HEIC_BRANDS.iter().any(|b| compat == *b) {
                        found_heic = true;
                    }
                    offset += 4;
                }
                // AVIF takes priority over HEIC when both present
                if found_avif {
                    return Some(ImageFormat::Avif);
                }
                if found_heic {
                    return Some(ImageFormat::Heic);
                }
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

        // BMP: "BM"
        if data.len() >= 2 && data[0] == b'B' && data[1] == b'M' {
            return Some(ImageFormat::Bmp);
        }

        // farbfeld: "farbfeld"
        if data.len() >= 8 && data[..8] == *b"farbfeld" {
            return Some(ImageFormat::Farbfeld);
        }

        // PNM family: P1-P7, Pf (grayscale PFM), PF (color PFM)
        if data.len() >= 2 && data[0] == b'P' {
            match data[1] {
                b'1'..=b'7' | b'F' | b'f' => return Some(ImageFormat::Pnm),
                _ => {}
            }
        }

        // TIFF: II (little-endian) or MM (big-endian) + magic number 42
        if data.len() >= 4 {
            if data[0] == b'I' && data[1] == b'I' && data[2] == 42 && data[3] == 0 {
                return Some(ImageFormat::Tiff);
            }
            if data[0] == b'M' && data[1] == b'M' && data[2] == 0 && data[3] == 42 {
                return Some(ImageFormat::Tiff);
            }
        }

        // ICO: 00 00 01 00 (icon) or 00 00 02 00 (cursor)
        if data.len() >= 4
            && data[0] == 0
            && data[1] == 0
            && (data[2] == 1 || data[2] == 2)
            && data[3] == 0
        {
            return Some(ImageFormat::Ico);
        }

        // QOI: "qoif"
        if data.len() >= 4 && data[..4] == *b"qoif" {
            return Some(ImageFormat::Qoi);
        }

        None
    }

    /// Detect format from file extension (case-insensitive).
    ///
    /// Only matches built-in formats. For custom formats, iterate your
    /// registered [`CustomImageFormat::extensions`] or use a codec registry.
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
            b"heic" | b"heif" | b"hif" => Some(ImageFormat::Heic),
            b"pnm" | b"ppm" | b"pgm" | b"pbm" | b"pam" | b"pfm" => Some(ImageFormat::Pnm),
            b"bmp" => Some(ImageFormat::Bmp),
            b"tiff" | b"tif" => Some(ImageFormat::Tiff),
            b"ico" | b"cur" => Some(ImageFormat::Ico),
            b"ff" => Some(ImageFormat::Farbfeld),
            b"qoi" => Some(ImageFormat::Qoi),
            _ => None,
        }
    }

    /// Primary MIME type string.
    pub fn mime_type(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Jxl => "image/jxl",
            ImageFormat::Heic => "image/heif",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Pnm => "image/x-portable-anymap",
            ImageFormat::Farbfeld => "image/x-farbfeld",
            ImageFormat::Qoi => "image/x-qoi",
            ImageFormat::Unknown => "application/octet-stream",
            ImageFormat::Custom(fmt) => fmt.preferred_mime_type,
        }
    }

    /// All recognized MIME types for this format.
    pub fn mime_types(self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["image/jpeg"],
            ImageFormat::Png => &["image/png"],
            ImageFormat::Gif => &["image/gif"],
            ImageFormat::WebP => &["image/webp"],
            ImageFormat::Avif => &["image/avif"],
            ImageFormat::Jxl => &["image/jxl"],
            ImageFormat::Heic => &["image/heif", "image/heic"],
            ImageFormat::Bmp => &["image/bmp", "image/x-bmp"],
            ImageFormat::Tiff => &["image/tiff"],
            ImageFormat::Ico => &["image/x-icon", "image/vnd.microsoft.icon"],
            ImageFormat::Pnm => &[
                "image/x-portable-anymap",
                "image/x-portable-pixmap",
                "image/x-portable-graymap",
                "image/x-portable-bitmap",
            ],
            ImageFormat::Farbfeld => &["image/x-farbfeld"],
            ImageFormat::Qoi => &["image/x-qoi"],
            ImageFormat::Unknown => &[],
            ImageFormat::Custom(fmt) => fmt.mime_types,
        }
    }

    /// Primary file extension (without dot).
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::WebP => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Jxl => "jxl",
            ImageFormat::Heic => "heif",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Ico => "ico",
            ImageFormat::Pnm => "pnm",
            ImageFormat::Farbfeld => "ff",
            ImageFormat::Qoi => "qoi",
            ImageFormat::Unknown => "bin",
            ImageFormat::Custom(fmt) => fmt.preferred_extension,
        }
    }

    /// All recognized file extensions.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            ImageFormat::Png => &["png"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::WebP => &["webp"],
            ImageFormat::Avif => &["avif"],
            ImageFormat::Jxl => &["jxl"],
            ImageFormat::Heic => &["heic", "heif", "hif"],
            ImageFormat::Bmp => &["bmp"],
            ImageFormat::Tiff => &["tiff", "tif"],
            ImageFormat::Ico => &["ico", "cur"],
            ImageFormat::Pnm => &["pnm", "ppm", "pgm", "pbm", "pam", "pfm"],
            ImageFormat::Farbfeld => &["ff"],
            ImageFormat::Qoi => &["qoi"],
            ImageFormat::Unknown => &[],
            ImageFormat::Custom(fmt) => fmt.extensions,
        }
    }

    /// Whether this format supports lossy encoding.
    pub fn supports_lossy(self) -> bool {
        match self {
            ImageFormat::Jpeg
            | ImageFormat::WebP
            | ImageFormat::Avif
            | ImageFormat::Jxl
            | ImageFormat::Heic => true,
            ImageFormat::Custom(fmt) => fmt.supports_lossy,
            _ => false,
        }
    }

    /// Whether this format supports lossless encoding.
    pub fn supports_lossless(self) -> bool {
        match self {
            ImageFormat::WebP
            | ImageFormat::Gif
            | ImageFormat::Png
            | ImageFormat::Avif
            | ImageFormat::Jxl
            | ImageFormat::Tiff
            | ImageFormat::Pnm
            | ImageFormat::Bmp
            | ImageFormat::Farbfeld
            | ImageFormat::Qoi => true,
            ImageFormat::Custom(fmt) => fmt.supports_lossless,
            _ => false,
        }
    }

    /// Whether this format supports animation.
    pub fn supports_animation(self) -> bool {
        match self {
            ImageFormat::Png
            | ImageFormat::WebP
            | ImageFormat::Gif
            | ImageFormat::Avif
            | ImageFormat::Jxl => true,
            ImageFormat::Custom(fmt) => fmt.supports_animation,
            _ => false,
        }
    }

    /// Recommended bytes to fetch for probing any format.
    ///
    /// 4096 bytes is enough for all built-in formats including JPEG (which
    /// may have large EXIF/APP segments before the SOF marker).
    pub const RECOMMENDED_PROBE_BYTES: usize = 4096;

    /// Minimum bytes needed for reliable magic byte detection.
    pub fn magic_bytes_needed(self) -> usize {
        match self {
            ImageFormat::Png => 33,      // 8 sig + 25 IHDR
            ImageFormat::Gif => 13,      // 6 header + 7 LSD
            ImageFormat::WebP => 30,     // RIFF(12) + chunk header + VP8X dims
            ImageFormat::Jpeg => 2048,   // SOF can follow large EXIF/APP segments
            ImageFormat::Avif => 512,    // ISOBMFF box traversal (ftyp + meta)
            ImageFormat::Heic => 512,    // ISOBMFF box traversal (ftyp + meta)
            ImageFormat::Jxl => 256,     // codestream header or container + jxlc
            ImageFormat::Pnm => 20,      // magic + ASCII dimensions
            ImageFormat::Bmp => 54,      // 14 file header + 40 info header
            ImageFormat::Tiff => 8,      // endian marker + magic + IFD offset
            ImageFormat::Ico => 22,      // 6 header + 16 first entry
            ImageFormat::Farbfeld => 16, // 8 magic + 4 width + 4 height
            ImageFormat::Qoi => 14,      // 4 magic + 4 width + 4 height + 1 channels + 1 colorspace
            ImageFormat::Unknown => 0,
            ImageFormat::Custom(fmt) => fmt.magic_bytes_needed,
        }
    }

    /// Whether this format supports alpha channel.
    pub fn supports_alpha(self) -> bool {
        match self {
            ImageFormat::Jpeg => false,
            ImageFormat::Custom(fmt) => fmt.supports_alpha,
            _ => true,
        }
    }
}

impl core::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::Gif => "GIF",
            ImageFormat::WebP => "WebP",
            ImageFormat::Avif => "AVIF",
            ImageFormat::Jxl => "JPEG XL",
            ImageFormat::Heic => "HEIC",
            ImageFormat::Bmp => "BMP",
            ImageFormat::Tiff => "TIFF",
            ImageFormat::Ico => "ICO",
            ImageFormat::Pnm => "PNM",
            ImageFormat::Farbfeld => "farbfeld",
            ImageFormat::Qoi => "QOI",
            ImageFormat::Unknown => "Unknown",
            ImageFormat::Custom(fmt) => fmt.display_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

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
    fn mime_types_primary() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Jxl.mime_type(), "image/jxl");
    }

    #[test]
    fn mime_types_all() {
        assert_eq!(ImageFormat::Jpeg.mime_types(), &["image/jpeg"]);
        assert!(ImageFormat::Heic.mime_types().contains(&"image/heif"));
        assert!(ImageFormat::Heic.mime_types().contains(&"image/heic"));
    }

    #[test]
    fn probe_constants() {
        assert_eq!(ImageFormat::RECOMMENDED_PROBE_BYTES, 4096);
        assert!(ImageFormat::Jpeg.magic_bytes_needed() > ImageFormat::Png.magic_bytes_needed());
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
        assert_eq!(ImageFormat::from_extension("tiff"), Some(ImageFormat::Tiff));
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
        assert!(ImageFormat::Png.supports_animation());

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
    fn pnm_magic_bytes_needed() {
        assert_eq!(ImageFormat::Pnm.magic_bytes_needed(), 20);
    }

    #[test]
    fn detect_bmp() {
        assert_eq!(ImageFormat::detect(b"BM\x00\x00"), Some(ImageFormat::Bmp));
    }

    #[test]
    fn detect_farbfeld() {
        assert_eq!(
            ImageFormat::detect(b"farbfeld\x00\x00\x00\x01\x00\x00\x00\x01"),
            Some(ImageFormat::Farbfeld)
        );
    }

    #[test]
    fn from_extension_bmp() {
        assert_eq!(ImageFormat::from_extension("bmp"), Some(ImageFormat::Bmp));
        assert_eq!(ImageFormat::from_extension("BMP"), Some(ImageFormat::Bmp));
    }

    #[test]
    fn from_extension_farbfeld() {
        assert_eq!(
            ImageFormat::from_extension("ff"),
            Some(ImageFormat::Farbfeld)
        );
    }

    #[test]
    fn bmp_capabilities() {
        assert!(!ImageFormat::Bmp.supports_lossy());
        assert!(ImageFormat::Bmp.supports_lossless());
        assert!(!ImageFormat::Bmp.supports_animation());
        assert!(ImageFormat::Bmp.supports_alpha());
    }

    #[test]
    fn farbfeld_capabilities() {
        assert!(!ImageFormat::Farbfeld.supports_lossy());
        assert!(ImageFormat::Farbfeld.supports_lossless());
        assert!(!ImageFormat::Farbfeld.supports_animation());
        assert!(ImageFormat::Farbfeld.supports_alpha());
    }

    #[test]
    fn bmp_display() {
        assert_eq!(alloc::format!("{}", ImageFormat::Bmp), "BMP");
    }

    #[test]
    fn farbfeld_display() {
        assert_eq!(alloc::format!("{}", ImageFormat::Farbfeld), "farbfeld");
    }

    #[test]
    fn bmp_mime_type() {
        assert_eq!(ImageFormat::Bmp.mime_type(), "image/bmp");
    }

    #[test]
    fn farbfeld_mime_type() {
        assert_eq!(ImageFormat::Farbfeld.mime_type(), "image/x-farbfeld");
    }

    #[test]
    fn bmp_extensions() {
        assert_eq!(ImageFormat::Bmp.extensions(), &["bmp"]);
    }

    #[test]
    fn farbfeld_extensions() {
        assert_eq!(ImageFormat::Farbfeld.extensions(), &["ff"]);
    }

    // --- HEIC tests ---

    #[test]
    fn detect_heic() {
        // ftyp box with heic major brand: [size=20][ftyp][heic][minor][no compat brands]
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes()); // box size
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"heic");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]); // minor version
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Heic));
    }

    #[test]
    fn detect_heic_heix_brand() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"heix");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Heic));
    }

    #[test]
    fn detect_heic_hevc_brand() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"hevc");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Heic));
    }

    #[test]
    fn detect_avif_still_works() {
        // Regression: AVIF detection must still work after HEIC addition
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Avif));

        data[8..12].copy_from_slice(b"avis");
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Avif));
    }

    #[test]
    fn detect_mif1_with_heic_compat() {
        // mif1 major brand + heic compatible brand → Heic
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes()); // box size = 24
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"mif1"); // major brand
        data[12..16].copy_from_slice(&[0, 0, 0, 0]); // minor version
        data[16..20].copy_from_slice(b"heic"); // compatible brand
        data[20..24].copy_from_slice(b"hevx"); // another compatible brand
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Heic));
    }

    #[test]
    fn detect_mif1_with_avif_compat() {
        // mif1 major brand + avif compatible brand → Avif
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"mif1");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        data[16..20].copy_from_slice(b"avif"); // compatible brand
        data[20..24].copy_from_slice(b"heic"); // also heic — but avif takes priority
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Avif));
    }

    #[test]
    fn detect_mif1_no_known_compat() {
        // mif1 with no recognized compatible brands → None
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"mif1");
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        data[16..20].copy_from_slice(b"xxxx"); // unknown brand
        assert_eq!(ImageFormat::detect(&data), None);
    }

    #[test]
    fn from_extension_heic() {
        assert_eq!(ImageFormat::from_extension("heic"), Some(ImageFormat::Heic));
        assert_eq!(ImageFormat::from_extension("heif"), Some(ImageFormat::Heic));
        assert_eq!(ImageFormat::from_extension("hif"), Some(ImageFormat::Heic));
        assert_eq!(ImageFormat::from_extension("HEIC"), Some(ImageFormat::Heic));
        assert_eq!(ImageFormat::from_extension("HEIF"), Some(ImageFormat::Heic));
    }

    #[test]
    fn heic_capabilities() {
        assert!(ImageFormat::Heic.supports_lossy());
        assert!(!ImageFormat::Heic.supports_lossless());
        assert!(!ImageFormat::Heic.supports_animation());
        assert!(ImageFormat::Heic.supports_alpha());
    }

    #[test]
    fn heif_display() {
        assert_eq!(alloc::format!("{}", ImageFormat::Heic), "HEIC");
    }

    #[test]
    fn heif_mime_type() {
        assert_eq!(ImageFormat::Heic.mime_type(), "image/heif");
    }

    #[test]
    fn heic_extensions() {
        let exts = ImageFormat::Heic.extensions();
        assert!(exts.contains(&"heic"));
        assert!(exts.contains(&"heif"));
        assert!(exts.contains(&"hif"));
    }

    #[test]
    fn heic_min_probe_bytes() {
        assert_eq!(ImageFormat::Heic.magic_bytes_needed(), 512);
    }

    // --- Custom format tests ---

    fn detect_test_format(data: &[u8]) -> bool {
        data.len() >= 4 && data[..4] == *b"TEST"
    }

    static TEST_FORMAT: CustomImageFormat = CustomImageFormat {
        name: "testformat",
        display_name: "Test Format",
        preferred_extension: "test",
        extensions: &["test", "tst"],
        preferred_mime_type: "image/x-test",
        mime_types: &["image/x-test", "application/x-test"],
        supports_alpha: true,
        supports_animation: false,
        supports_lossless: true,
        supports_lossy: false,
        magic_bytes_needed: 4,
        detect: detect_test_format,
    };

    static TEST_FORMAT_2: CustomImageFormat = CustomImageFormat {
        name: "testformat",
        display_name: "Test Format 2",
        preferred_extension: "tf2",
        extensions: &["tf2"],
        preferred_mime_type: "image/x-test2",
        mime_types: &["image/x-test2"],
        supports_alpha: false,
        supports_animation: false,
        supports_lossless: false,
        supports_lossy: false,
        magic_bytes_needed: 0,
        detect: |_| false,
    };

    #[test]
    fn custom_format_metadata() {
        let fmt = ImageFormat::Custom(&TEST_FORMAT);
        assert_eq!(fmt.mime_type(), "image/x-test");
        assert_eq!(fmt.mime_types(), &["image/x-test", "application/x-test"]);
        assert_eq!(fmt.extension(), "test");
        assert_eq!(fmt.extensions(), &["test", "tst"]);
        assert!(fmt.supports_alpha());
        assert!(!fmt.supports_animation());
        assert!(fmt.supports_lossless());
        assert!(!fmt.supports_lossy());
        assert_eq!(fmt.magic_bytes_needed(), 4);
    }

    #[test]
    fn custom_format_display() {
        let fmt = ImageFormat::Custom(&TEST_FORMAT);
        assert_eq!(alloc::format!("{fmt}"), "Test Format");
    }

    #[test]
    fn custom_format_detect() {
        assert!((TEST_FORMAT.detect)(b"TESTdata"));
        assert!(!(TEST_FORMAT.detect)(b"NOPE"));
    }

    #[test]
    fn custom_format_equality_by_name() {
        // Same name → equal, even though other fields differ
        let a = ImageFormat::Custom(&TEST_FORMAT);
        let b = ImageFormat::Custom(&TEST_FORMAT_2);
        assert_eq!(a, b);

        // Different name → not equal
        static OTHER: CustomImageFormat = CustomImageFormat {
            name: "other",
            display_name: "Other",
            preferred_extension: "oth",
            extensions: &["oth"],
            preferred_mime_type: "image/x-other",
            mime_types: &["image/x-other"],
            supports_alpha: false,
            supports_animation: false,
            supports_lossless: false,
            supports_lossy: false,
            magic_bytes_needed: 0,
            detect: |_| false,
        };
        assert_ne!(a, ImageFormat::Custom(&OTHER));
    }

    #[test]
    fn custom_format_hash() {
        use core::hash::{Hash, Hasher};
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 { self.0 }
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
        }
        fn hash_of(fmt: ImageFormat) -> u64 {
            let mut hasher = SimpleHasher(0);
            fmt.hash(&mut hasher);
            hasher.finish()
        }
        // Same name → same hash
        assert_eq!(
            hash_of(ImageFormat::Custom(&TEST_FORMAT)),
            hash_of(ImageFormat::Custom(&TEST_FORMAT_2))
        );
    }

    #[test]
    fn custom_not_equal_to_builtin() {
        let custom = ImageFormat::Custom(&TEST_FORMAT);
        assert_ne!(custom, ImageFormat::Jpeg);
        assert_ne!(custom, ImageFormat::Unknown);
    }

    #[test]
    fn custom_format_is_copy() {
        let a = ImageFormat::Custom(&TEST_FORMAT);
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
