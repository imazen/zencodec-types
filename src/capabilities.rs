//! Codec capability descriptors.
//!
//! Each codec returns a static [`CodecCapabilities`] describing what it
//! supports. This lets callers discover behavior before calling methods
//! that might be no-ops or expensive.

/// Describes what a codec supports.
///
/// Returned by [`Encoding::capabilities()`](crate::Encoding::capabilities) and
/// [`Decoding::capabilities()`](crate::Decoding::capabilities) as a `&'static`
/// reference. The struct uses getter methods so fields can be added over time
/// without breaking changes.
///
/// # Example
///
/// ```
/// use zencodec_types::CodecCapabilities;
///
/// static CAPS: CodecCapabilities = CodecCapabilities::new()
///     .with_encode_icc(true)
///     .with_encode_exif(true)
///     .with_encode_xmp(true)
///     .with_encode_cancel(true)
///     .with_decode_cancel(true)
///     .with_native_gray(true)
///     .with_cheap_probe(true);
///
/// assert!(CAPS.encode_icc());
/// assert!(CAPS.cheap_probe());
/// ```
#[non_exhaustive]
pub struct CodecCapabilities {
    encode_icc: bool,
    encode_exif: bool,
    encode_xmp: bool,
    decode_icc: bool,
    decode_exif: bool,
    decode_xmp: bool,
    encode_cancel: bool,
    decode_cancel: bool,
    native_gray: bool,
    cheap_probe: bool,
    encode_animation: bool,
    decode_animation: bool,
    native_16bit: bool,
    lossless: bool,
    hdr: bool,
    encode_cicp: bool,
    decode_cicp: bool,
    enforces_max_pixels: bool,
    enforces_max_memory: bool,
    enforces_max_file_size: bool,
}

impl Default for CodecCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl CodecCapabilities {
    /// Create capabilities with everything disabled.
    pub const fn new() -> Self {
        Self {
            encode_icc: false,
            encode_exif: false,
            encode_xmp: false,
            decode_icc: false,
            decode_exif: false,
            decode_xmp: false,
            encode_cancel: false,
            decode_cancel: false,
            native_gray: false,
            cheap_probe: false,
            encode_animation: false,
            decode_animation: false,
            native_16bit: false,
            lossless: false,
            hdr: false,
            encode_cicp: false,
            decode_cicp: false,
            enforces_max_pixels: false,
            enforces_max_memory: false,
            enforces_max_file_size: false,
        }
    }

    /// Whether the encoder embeds ICC color profiles from `with_metadata`.
    pub const fn encode_icc(&self) -> bool {
        self.encode_icc
    }

    /// Whether the encoder embeds EXIF data from `with_metadata`.
    pub const fn encode_exif(&self) -> bool {
        self.encode_exif
    }

    /// Whether the encoder embeds XMP data from `with_metadata`.
    pub const fn encode_xmp(&self) -> bool {
        self.encode_xmp
    }

    /// Whether the decoder extracts ICC color profiles into `ImageInfo`.
    pub const fn decode_icc(&self) -> bool {
        self.decode_icc
    }

    /// Whether the decoder extracts EXIF data into `ImageInfo`.
    pub const fn decode_exif(&self) -> bool {
        self.decode_exif
    }

    /// Whether the decoder extracts XMP data into `ImageInfo`.
    pub const fn decode_xmp(&self) -> bool {
        self.decode_xmp
    }

    /// Whether `with_stop` on encode jobs is respected (not a no-op).
    pub const fn encode_cancel(&self) -> bool {
        self.encode_cancel
    }

    /// Whether `with_stop` on decode jobs is respected (not a no-op).
    pub const fn decode_cancel(&self) -> bool {
        self.decode_cancel
    }

    /// Whether the codec supports grayscale natively (without expanding to RGB).
    pub const fn native_gray(&self) -> bool {
        self.native_gray
    }

    /// Whether `probe_header` is cheap (header parse only, not a full decode).
    pub const fn cheap_probe(&self) -> bool {
        self.cheap_probe
    }

    /// Whether the codec supports encoding animation (multiple frames).
    pub const fn encode_animation(&self) -> bool {
        self.encode_animation
    }

    /// Whether the codec supports decoding animation (multiple frames).
    pub const fn decode_animation(&self) -> bool {
        self.decode_animation
    }

    /// Whether the codec supports 16-bit per channel natively (without
    /// dithering/truncating to 8-bit internally).
    pub const fn native_16bit(&self) -> bool {
        self.native_16bit
    }

    /// Whether the codec supports mathematically lossless encoding.
    pub const fn lossless(&self) -> bool {
        self.lossless
    }

    /// Whether the codec supports HDR content (wide gamut, high bit depth,
    /// PQ/HLG transfer functions, HDR metadata).
    pub const fn hdr(&self) -> bool {
        self.hdr
    }

    /// Whether the encoder embeds CICP color description from `with_metadata`.
    pub const fn encode_cicp(&self) -> bool {
        self.encode_cicp
    }

    /// Whether the decoder extracts CICP color description into `ImageInfo`.
    pub const fn decode_cicp(&self) -> bool {
        self.decode_cicp
    }

    /// Whether the codec enforces [`ResourceLimits::max_pixels`](crate::ResourceLimits::max_pixels).
    pub const fn enforces_max_pixels(&self) -> bool {
        self.enforces_max_pixels
    }

    /// Whether the codec enforces [`ResourceLimits::max_memory_bytes`](crate::ResourceLimits::max_memory_bytes).
    pub const fn enforces_max_memory(&self) -> bool {
        self.enforces_max_memory
    }

    /// Whether the codec enforces [`ResourceLimits::max_file_size`](crate::ResourceLimits::max_file_size).
    pub const fn enforces_max_file_size(&self) -> bool {
        self.enforces_max_file_size
    }

    // --- const builder methods for static construction ---

    /// Set ICC embed support on encode.
    pub const fn with_encode_icc(mut self, v: bool) -> Self {
        self.encode_icc = v;
        self
    }

    /// Set EXIF embed support on encode.
    pub const fn with_encode_exif(mut self, v: bool) -> Self {
        self.encode_exif = v;
        self
    }

    /// Set XMP embed support on encode.
    pub const fn with_encode_xmp(mut self, v: bool) -> Self {
        self.encode_xmp = v;
        self
    }

    /// Set ICC extraction support on decode.
    pub const fn with_decode_icc(mut self, v: bool) -> Self {
        self.decode_icc = v;
        self
    }

    /// Set EXIF extraction support on decode.
    pub const fn with_decode_exif(mut self, v: bool) -> Self {
        self.decode_exif = v;
        self
    }

    /// Set XMP extraction support on decode.
    pub const fn with_decode_xmp(mut self, v: bool) -> Self {
        self.decode_xmp = v;
        self
    }

    /// Set cooperative cancellation support on encode.
    pub const fn with_encode_cancel(mut self, v: bool) -> Self {
        self.encode_cancel = v;
        self
    }

    /// Set cooperative cancellation support on decode.
    pub const fn with_decode_cancel(mut self, v: bool) -> Self {
        self.decode_cancel = v;
        self
    }

    /// Set native grayscale support.
    pub const fn with_native_gray(mut self, v: bool) -> Self {
        self.native_gray = v;
        self
    }

    /// Set whether probe_header is cheap.
    pub const fn with_cheap_probe(mut self, v: bool) -> Self {
        self.cheap_probe = v;
        self
    }

    /// Set animation encoding support.
    pub const fn with_encode_animation(mut self, v: bool) -> Self {
        self.encode_animation = v;
        self
    }

    /// Set animation decoding support.
    pub const fn with_decode_animation(mut self, v: bool) -> Self {
        self.decode_animation = v;
        self
    }

    /// Set native 16-bit support.
    pub const fn with_native_16bit(mut self, v: bool) -> Self {
        self.native_16bit = v;
        self
    }

    /// Set lossless encoding support.
    pub const fn with_lossless(mut self, v: bool) -> Self {
        self.lossless = v;
        self
    }

    /// Set HDR support.
    pub const fn with_hdr(mut self, v: bool) -> Self {
        self.hdr = v;
        self
    }

    /// Set CICP embed support on encode.
    pub const fn with_encode_cicp(mut self, v: bool) -> Self {
        self.encode_cicp = v;
        self
    }

    /// Set CICP extraction support on decode.
    pub const fn with_decode_cicp(mut self, v: bool) -> Self {
        self.decode_cicp = v;
        self
    }

    /// Set whether the codec enforces max_pixels limits.
    pub const fn with_enforces_max_pixels(mut self, v: bool) -> Self {
        self.enforces_max_pixels = v;
        self
    }

    /// Set whether the codec enforces max_memory limits.
    pub const fn with_enforces_max_memory(mut self, v: bool) -> Self {
        self.enforces_max_memory = v;
        self
    }

    /// Set whether the codec enforces max_file_size limits.
    pub const fn with_enforces_max_file_size(mut self, v: bool) -> Self {
        self.enforces_max_file_size = v;
        self
    }
}

impl core::fmt::Debug for CodecCapabilities {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CodecCapabilities")
            .field("encode_icc", &self.encode_icc)
            .field("encode_exif", &self.encode_exif)
            .field("encode_xmp", &self.encode_xmp)
            .field("decode_icc", &self.decode_icc)
            .field("decode_exif", &self.decode_exif)
            .field("decode_xmp", &self.decode_xmp)
            .field("encode_cancel", &self.encode_cancel)
            .field("decode_cancel", &self.decode_cancel)
            .field("native_gray", &self.native_gray)
            .field("cheap_probe", &self.cheap_probe)
            .field("encode_animation", &self.encode_animation)
            .field("decode_animation", &self.decode_animation)
            .field("native_16bit", &self.native_16bit)
            .field("lossless", &self.lossless)
            .field("hdr", &self.hdr)
            .field("encode_cicp", &self.encode_cicp)
            .field("decode_cicp", &self.decode_cicp)
            .field("enforces_max_pixels", &self.enforces_max_pixels)
            .field("enforces_max_memory", &self.enforces_max_memory)
            .field("enforces_max_file_size", &self.enforces_max_file_size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_false() {
        let caps = CodecCapabilities::new();
        assert!(!caps.encode_icc());
        assert!(!caps.encode_exif());
        assert!(!caps.encode_xmp());
        assert!(!caps.decode_icc());
        assert!(!caps.decode_exif());
        assert!(!caps.decode_xmp());
        assert!(!caps.encode_cancel());
        assert!(!caps.decode_cancel());
        assert!(!caps.native_gray());
        assert!(!caps.cheap_probe());
        assert!(!caps.encode_animation());
        assert!(!caps.decode_animation());
        assert!(!caps.native_16bit());
        assert!(!caps.lossless());
        assert!(!caps.hdr());
        assert!(!caps.encode_cicp());
        assert!(!caps.decode_cicp());
        assert!(!caps.enforces_max_pixels());
        assert!(!caps.enforces_max_memory());
        assert!(!caps.enforces_max_file_size());
    }

    #[test]
    fn builder_sets_fields() {
        let caps = CodecCapabilities::new()
            .with_encode_icc(true)
            .with_decode_cancel(true)
            .with_native_gray(true)
            .with_cheap_probe(true)
            .with_encode_animation(true)
            .with_native_16bit(true)
            .with_hdr(true);
        assert!(caps.encode_icc());
        assert!(!caps.encode_exif());
        assert!(caps.decode_cancel());
        assert!(caps.native_gray());
        assert!(caps.cheap_probe());
        assert!(caps.encode_animation());
        assert!(!caps.decode_animation());
        assert!(caps.native_16bit());
        assert!(!caps.lossless());
        assert!(caps.hdr());
    }

    #[test]
    fn static_construction() {
        static CAPS: CodecCapabilities = CodecCapabilities::new()
            .with_encode_icc(true)
            .with_encode_exif(true)
            .with_encode_xmp(true)
            .with_encode_cancel(true)
            .with_decode_cancel(true)
            .with_encode_animation(true)
            .with_decode_animation(true)
            .with_lossless(true)
            .with_encode_cicp(true)
            .with_decode_cicp(true);
        assert!(CAPS.encode_icc());
        assert!(CAPS.encode_cancel());
        assert!(!CAPS.native_gray());
        assert!(CAPS.encode_animation());
        assert!(CAPS.decode_animation());
        assert!(CAPS.lossless());
        assert!(CAPS.encode_cicp());
        assert!(CAPS.decode_cicp());
    }

    #[test]
    fn enforces_limits_flags() {
        let caps = CodecCapabilities::new()
            .with_enforces_max_pixels(true)
            .with_enforces_max_memory(true)
            .with_enforces_max_file_size(true);
        assert!(caps.enforces_max_pixels());
        assert!(caps.enforces_max_memory());
        assert!(caps.enforces_max_file_size());
    }

    #[test]
    fn enforces_limits_static() {
        static CAPS: CodecCapabilities = CodecCapabilities::new()
            .with_enforces_max_pixels(true)
            .with_enforces_max_file_size(true);
        assert!(CAPS.enforces_max_pixels());
        assert!(!CAPS.enforces_max_memory());
        assert!(CAPS.enforces_max_file_size());
    }
}
