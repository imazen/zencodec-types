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
    }

    #[test]
    fn builder_sets_fields() {
        let caps = CodecCapabilities::new()
            .with_encode_icc(true)
            .with_decode_cancel(true)
            .with_native_gray(true)
            .with_cheap_probe(true);
        assert!(caps.encode_icc());
        assert!(!caps.encode_exif());
        assert!(caps.decode_cancel());
        assert!(caps.native_gray());
        assert!(caps.cheap_probe());
    }

    #[test]
    fn static_construction() {
        static CAPS: CodecCapabilities = CodecCapabilities::new()
            .with_encode_icc(true)
            .with_encode_exif(true)
            .with_encode_xmp(true)
            .with_encode_cancel(true)
            .with_decode_cancel(true);
        assert!(CAPS.encode_icc());
        assert!(CAPS.encode_cancel());
        assert!(!CAPS.native_gray());
    }
}
