//! Scanline-level streaming traits for pull-based decode and push-based encode.
//!
//! These traits complement the one-shot [`Decoder`] / [`Encoder`] traits with
//! incremental row-level access. Not every codec supports scanline streaming —
//! check [`CodecCapabilities::scanline_decode()`] / [`scanline_encode()`] or
//! test whether the codec's job type implements [`ScanlineDecodeJob`] /
//! [`ScanlineEncodeJob`].
//!
//! ```text
//!                                 ┌→ Decoder       (one-shot)
//! DecoderConfig → DecodeJob<'a>  ┼→ FrameDecoder   (animation)
//!                                 └→ ScanlineDecoder (streaming pull)   ← NEW
//!
//!                                 ┌→ Encoder       (one-shot or push_rows)
//! EncoderConfig → EncodeJob<'a>  ┼→ FrameEncoder   (animation)
//!                                 └→ ScanlineEncoder (streaming push)   ← NEW
//! ```
//!
//! # Decode: pull rows into caller buffer
//!
//! The caller creates a [`ScanlineDecoder`] from encoded data via
//! [`ScanlineDecodeJob::scanline_decoder()`]. The decoder parses the header
//! and then produces rows on demand:
//!
//! ```text
//! let decoder = config.job().scanline_decoder(data)?;
//! let info = decoder.image_info();
//! let desc = decoder.output_descriptor();
//! let strip_h = decoder.preferred_strip_height();
//!
//! let mut buf = PixelBuffer::new(info.width, strip_h, desc);
//! loop {
//!     let rows = decoder.read_rows(buf.as_slice_mut())?;
//!     if rows == 0 { break; }
//!     // process `rows` scanlines in `buf`...
//! }
//! ```
//!
//! # Encode: push rows from caller
//!
//! The caller creates a [`ScanlineEncoder`] with the image dimensions via
//! [`ScanlineEncodeJob::scanline_encoder()`], then pushes rows incrementally:
//!
//! ```text
//! let mut encoder = config.job()
//!     .with_metadata(&meta)
//!     .scanline_encoder(width, height, descriptor)?;
//! let strip_h = encoder.preferred_strip_height();
//!
//! for strip in pipeline.strips() {
//!     encoder.write_rows(strip)?;
//! }
//! let output = encoder.finish()?;
//! ```
//!
//! # Which codecs support this?
//!
//! | Codec | Scanline decode | Scanline encode | Notes |
//! |-------|:-:|:-:|---|
//! | JPEG  | Yes | Yes | MCU-row granularity (8 or 16 rows) |
//! | PNG   | Yes | Yes | Row-at-a-time defiltering |
//! | GIF   | No  | No  | Frame-at-once (LZW + palette) |
//! | WebP  | No  | No  | Frame-at-once (VP8/VP8L) |
//! | AVIF  | No  | No  | Frame-at-once (AV1 tiles) |
//! | JXL   | Possible | Possible | Group-level streaming (future) |

use crate::buffer::PixelDescriptor;
use crate::output::EncodeOutput;
use crate::{DecodeJob, EncodeJob, ImageInfo, PixelSlice, PixelSliceMut};

// ===========================================================================
// Scanline decode (pull-based)
// ===========================================================================

/// Pull-based scanline decoder for streaming pipelines.
///
/// Parses the header upfront, then produces decoded rows on demand.
/// The caller provides a destination buffer and the decoder fills it
/// with the next batch of scanlines.
///
/// # Strip ordering
///
/// Rows are produced in top-to-bottom order. The decoder tracks its
/// current position internally. Unless [`supports_seek()`](ScanlineDecoder::supports_seek)
/// returns true, you cannot go backwards.
///
/// # Buffer requirements
///
/// The destination buffer passed to [`read_rows()`](ScanlineDecoder::read_rows) must:
/// - Have width equal to `image_info().width`
/// - Have a `PixelDescriptor` compatible with [`output_descriptor()`](ScanlineDecoder::output_descriptor)
/// - Have at least 1 row of capacity (more rows = fewer calls = better perf)
///
/// For best performance, use [`preferred_strip_height()`](ScanlineDecoder::preferred_strip_height)
/// rows. JPEG decoders prefer MCU-aligned heights (8 or 16 rows); PNG
/// decoders prefer 1 row.
pub trait ScanlineDecoder: Send {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Image metadata from the parsed header.
    ///
    /// Includes dimensions, format, color info (CICP, ICC), orientation,
    /// and any embedded metadata (EXIF, XMP). Available immediately after
    /// creation — no decoding required.
    fn image_info(&self) -> &ImageInfo;

    /// Pixel format of decoded output rows.
    ///
    /// All rows from [`read_rows()`](ScanlineDecoder::read_rows) will use this
    /// format. Must be one of the codec's
    /// [`supported_descriptors()`](crate::DecoderConfig::supported_descriptors).
    fn output_descriptor(&self) -> PixelDescriptor;

    /// Suggested strip height for optimal decoding performance.
    ///
    /// For JPEG, typically the MCU height (8 or 16 rows).
    /// For PNG, typically 1 (row-at-a-time defiltering).
    ///
    /// This is advisory — callers may use any height. Using a multiple of
    /// the preferred height avoids partial-MCU buffering in JPEG.
    fn preferred_strip_height(&self) -> u32;

    /// Number of rows not yet decoded.
    ///
    /// Starts at `image_info().height` and decreases with each
    /// [`read_rows()`](ScanlineDecoder::read_rows) call. Returns 0 when
    /// the image is fully decoded.
    fn rows_remaining(&self) -> u32;

    /// Read the next strip of scanlines into `dst`.
    ///
    /// Returns the number of rows actually written. Returns 0 when
    /// the image is fully decoded. May return fewer rows than the
    /// buffer height at the end of the image.
    ///
    /// The buffer's `PixelDescriptor` must be compatible with
    /// [`output_descriptor()`](ScanlineDecoder::output_descriptor).
    /// The buffer width must equal `image_info().width`.
    fn read_rows(&mut self, dst: PixelSliceMut<'_>) -> Result<u32, Self::Error>;

    /// Whether random-access seeking is supported.
    ///
    /// Most decoders return `false` — they can only produce rows in order.
    /// JPEG decoders with restart markers may support seeking.
    fn supports_seek(&self) -> bool {
        false
    }

    /// Seek to a specific row for random access.
    ///
    /// Only meaningful if [`supports_seek()`](ScanlineDecoder::supports_seek)
    /// returns true. Seeking backwards resets the decode position. Seeking
    /// forward skips intervening rows (the decoder may decode and discard
    /// them internally).
    ///
    /// After seeking, the next [`read_rows()`](ScanlineDecoder::read_rows)
    /// call produces rows starting at `y`.
    fn seek_to_row(&mut self, y: u32) -> Result<(), Self::Error>;
}

// ===========================================================================
// Scanline encode (push-based)
// ===========================================================================

/// Push-based scanline encoder for streaming pipelines.
///
/// Initialized with full image dimensions upfront (for header writing),
/// then receives rows incrementally. Call [`finish()`](ScanlineEncoder::finish)
/// after all rows have been pushed to get the encoded output.
///
/// # Strip ordering
///
/// Rows must be pushed in top-to-bottom order via
/// [`write_rows()`](ScanlineEncoder::write_rows). The total number of rows
/// pushed must equal the height given at creation. Each call's `PixelSlice`
/// must have the same width and `PixelDescriptor` as specified at creation.
///
/// # Performance
///
/// For best performance, push strips of [`preferred_strip_height()`](ScanlineEncoder::preferred_strip_height)
/// rows at a time. JPEG encoders prefer MCU-aligned heights (8 or 16 rows);
/// PNG encoders are flexible.
pub trait ScanlineEncoder: Send {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Suggested strip height for optimal encoding performance.
    ///
    /// For JPEG, typically the MCU height (8 or 16 rows).
    /// For PNG, typically 1 or more rows.
    ///
    /// This is advisory — callers may push any number of rows per call.
    fn preferred_strip_height(&self) -> u32;

    /// Push scanline rows to the encoder.
    ///
    /// Rows must arrive in top-to-bottom order. Each call's `PixelSlice`
    /// must have the same width and `PixelDescriptor` as specified when
    /// creating the encoder.
    ///
    /// The encoder writes headers and compressed data incrementally where
    /// possible (JPEG, PNG). Codecs that need full-frame data (AVIF)
    /// buffer internally.
    fn write_rows(&mut self, rows: PixelSlice<'_>) -> Result<(), Self::Error>;

    /// Finalize encoding and return the encoded output.
    ///
    /// Must be called after all rows have been pushed. The total number
    /// of rows pushed must equal the height specified at creation.
    fn finish(self) -> Result<EncodeOutput, Self::Error>;
}

// ===========================================================================
// Job extension traits (factory methods)
// ===========================================================================

/// Extension for [`DecodeJob`]s that support pull-based scanline decoding.
///
/// Codecs that support scanline-level streaming implement this on their
/// job type. Frame-at-once codecs (WebP, AVIF, GIF) do not implement this.
///
/// # Example
///
/// ```ignore
/// use zencodec_types::{DecoderConfig, ScanlineDecodeJob};
///
/// let config = zenjpeg::DecoderConfig::default();
/// let mut decoder = config.job().scanline_decoder(data)?;
/// let info = decoder.image_info();
/// // ... read_rows() loop ...
/// ```
pub trait ScanlineDecodeJob<'a>: DecodeJob<'a> {
    /// The scanline decoder type produced by this job.
    type ScanlineDecoder: ScanlineDecoder<Error = Self::Error>;

    /// Create a pull-based scanline decoder from encoded data.
    ///
    /// Parses the image header and prepares for incremental row decoding.
    /// The decoder inherits stop token and resource limits from the job.
    ///
    /// The implementation may copy the data internally (JPEG needs the
    /// full compressed stream for Huffman decoding) or borrow it (PNG
    /// can stream from the input).
    fn scanline_decoder(self, data: &[u8]) -> Result<Self::ScanlineDecoder, Self::Error>;
}

/// Extension for [`EncodeJob`]s that support push-based scanline encoding.
///
/// Codecs that support scanline-level streaming implement this on their
/// job type. Frame-at-once codecs (WebP, AVIF, GIF) do not implement this.
///
/// # Example
///
/// ```ignore
/// use zencodec_types::{EncoderConfig, ScanlineEncodeJob, PixelDescriptor};
///
/// let config = zenjpeg::EncoderConfig::default()
///     .with_calibrated_quality(85.0);
/// let mut encoder = config.job()
///     .with_metadata(&metadata)
///     .scanline_encoder(width, height, PixelDescriptor::RGB8_SRGB)?;
/// // ... write_rows() loop ...
/// let output = encoder.finish()?;
/// ```
pub trait ScanlineEncodeJob<'a>: EncodeJob<'a> {
    /// The scanline encoder type produced by this job.
    type ScanlineEncoder: ScanlineEncoder<Error = Self::Error>;

    /// Create a push-based scanline encoder.
    ///
    /// Initializes the encoder with the full image dimensions and pixel
    /// format. The encoder writes format headers immediately (JPEG SOI/SOF,
    /// PNG IHDR) and is ready to receive rows.
    ///
    /// The encoder inherits stop token, metadata, and resource limits from
    /// the job.
    ///
    /// # Arguments
    ///
    /// * `width` — Image width in pixels
    /// * `height` — Image height in pixels
    /// * `descriptor` — Pixel format of the rows that will be pushed.
    ///   Must be one of the codec's
    ///   [`supported_descriptors()`](crate::EncoderConfig::supported_descriptors).
    fn scanline_encoder(
        self,
        width: u32,
        height: u32,
        descriptor: PixelDescriptor,
    ) -> Result<Self::ScanlineEncoder, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use crate::{EncodeOutput, ImageFormat, ImageInfo, PixelDescriptor};

    // A mock ScanlineDecoder for testing the trait API.
    struct MockScanlineDecoder {
        info: ImageInfo,
        descriptor: PixelDescriptor,
        rows_remaining: u32,
        row_data: Vec<u8>,
    }

    #[derive(Debug)]
    struct MockError(alloc::string::String);
    impl core::fmt::Display for MockError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "mock error: {}", self.0)
        }
    }
    impl core::error::Error for MockError {}

    impl MockScanlineDecoder {
        fn new(width: u32, height: u32) -> Self {
            let bpp = PixelDescriptor::RGB8.bytes_per_pixel() as usize;
            Self {
                info: ImageInfo::new(width, height, ImageFormat::Jpeg),
                descriptor: PixelDescriptor::RGB8,
                rows_remaining: height,
                row_data: vec![128u8; width as usize * bpp],
            }
        }
    }

    impl ScanlineDecoder for MockScanlineDecoder {
        type Error = MockError;

        fn image_info(&self) -> &ImageInfo {
            &self.info
        }

        fn output_descriptor(&self) -> PixelDescriptor {
            self.descriptor
        }

        fn preferred_strip_height(&self) -> u32 {
            8 // JPEG MCU height
        }

        fn rows_remaining(&self) -> u32 {
            self.rows_remaining
        }

        fn read_rows(&mut self, mut dst: PixelSliceMut<'_>) -> Result<u32, Self::Error> {
            if self.rows_remaining == 0 {
                return Ok(0);
            }
            let rows_to_write = dst.rows().min(self.rows_remaining);
            let bpp = self.descriptor.bytes_per_pixel() as usize;
            for y in 0..rows_to_write {
                let dst_row = dst.row_mut(y);
                let row_bytes = self.info.width as usize * bpp;
                dst_row[..row_bytes].copy_from_slice(&self.row_data[..row_bytes]);
            }
            self.rows_remaining -= rows_to_write;
            Ok(rows_to_write)
        }

        fn seek_to_row(&mut self, _y: u32) -> Result<(), Self::Error> {
            Err(MockError("seek not supported".into()))
        }
    }

    // A mock ScanlineEncoder for testing the trait API.
    struct MockScanlineEncoder {
        width: u32,
        height: u32,
        descriptor: PixelDescriptor,
        rows_received: u32,
        output: Vec<u8>,
    }

    impl MockScanlineEncoder {
        fn new(width: u32, height: u32, descriptor: PixelDescriptor) -> Self {
            Self {
                width,
                height,
                descriptor,
                rows_received: 0,
                output: Vec::new(),
            }
        }
    }

    impl ScanlineEncoder for MockScanlineEncoder {
        type Error = MockError;

        fn preferred_strip_height(&self) -> u32 {
            8
        }

        fn write_rows(&mut self, rows: PixelSlice<'_>) -> Result<(), Self::Error> {
            if rows.width() != self.width {
                return Err(MockError("width mismatch".into()));
            }
            if rows.descriptor() != self.descriptor {
                return Err(MockError("descriptor mismatch".into()));
            }
            self.rows_received += rows.rows();
            if self.rows_received > self.height {
                return Err(MockError("too many rows".into()));
            }
            // Accumulate raw pixel data (mock)
            let bpp = self.descriptor.bytes_per_pixel() as usize;
            for y in 0..rows.rows() {
                let row = rows.row(y);
                self.output.extend_from_slice(&row[..self.width as usize * bpp]);
            }
            Ok(())
        }

        fn finish(self) -> Result<EncodeOutput, Self::Error> {
            if self.rows_received != self.height {
                return Err(MockError(alloc::format!(
                    "expected {} rows, got {}",
                    self.height, self.rows_received
                )));
            }
            Ok(EncodeOutput::new(self.output, ImageFormat::Jpeg))
        }
    }

    #[test]
    fn scanline_decoder_basic() {
        let mut decoder = MockScanlineDecoder::new(16, 32);
        assert_eq!(decoder.image_info().width, 16);
        assert_eq!(decoder.image_info().height, 32);
        assert_eq!(decoder.rows_remaining(), 32);
        assert_eq!(decoder.preferred_strip_height(), 8);
        assert!(!decoder.supports_seek());
        assert!(decoder.seek_to_row(0).is_err());

        // Read strips
        let desc = decoder.output_descriptor();
        let bpp = desc.bytes_per_pixel() as usize;
        let stride = 16 * bpp;
        let mut buf = vec![0u8; stride * 8];

        let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 8);
        assert_eq!(decoder.rows_remaining(), 24);

        // Read remaining
        let mut total_rows = rows;
        while decoder.rows_remaining() > 0 {
            let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
                .expect("valid buffer");
            let rows = decoder.read_rows(slice).unwrap();
            assert!(rows > 0);
            total_rows += rows;
        }
        assert_eq!(total_rows, 32);

        // Verify EOF
        let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn scanline_encoder_basic() {
        let descriptor = PixelDescriptor::RGB8;
        let mut encoder = MockScanlineEncoder::new(16, 32, descriptor);
        assert_eq!(encoder.preferred_strip_height(), 8);

        let bpp = descriptor.bytes_per_pixel() as usize;
        let stride = 16 * bpp;
        let data = vec![200u8; stride * 8];

        // Push 4 strips of 8 rows each
        for _ in 0..4 {
            let slice = PixelSlice::new(&data, 16, 8, stride, descriptor)
                .expect("valid buffer");
            encoder.write_rows(slice).unwrap();
        }

        let output = encoder.finish().unwrap();
        assert_eq!(output.format(), ImageFormat::Jpeg);
        assert_eq!(output.len(), 16 * 32 * bpp);
    }

    #[test]
    fn scanline_encoder_rejects_width_mismatch() {
        let descriptor = PixelDescriptor::RGB8;
        let mut encoder = MockScanlineEncoder::new(16, 8, descriptor);

        let bpp = descriptor.bytes_per_pixel() as usize;
        let wrong_stride = 32 * bpp;
        let data = vec![0u8; wrong_stride * 8];
        let slice = PixelSlice::new(&data, 32, 8, wrong_stride, descriptor)
            .expect("valid buffer");
        assert!(encoder.write_rows(slice).is_err());
    }

    #[test]
    fn scanline_encoder_rejects_incomplete() {
        let descriptor = PixelDescriptor::RGB8;
        let mut encoder = MockScanlineEncoder::new(16, 32, descriptor);

        let bpp = descriptor.bytes_per_pixel() as usize;
        let stride = 16 * bpp;
        let data = vec![0u8; stride * 8];
        let slice = PixelSlice::new(&data, 16, 8, stride, descriptor)
            .expect("valid buffer");

        // Push only 1 strip (8 rows) instead of 4 (32 rows)
        encoder.write_rows(slice).unwrap();
        assert!(encoder.finish().is_err());
    }

    #[test]
    fn scanline_decoder_partial_last_strip() {
        // Image height 10, strip height 8 → last strip has 2 rows
        let mut decoder = MockScanlineDecoder::new(4, 10);
        let desc = decoder.output_descriptor();
        let bpp = desc.bytes_per_pixel() as usize;
        let stride = 4 * bpp;
        let mut buf = vec![0u8; stride * 8];

        // First strip: 8 rows
        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 8);
        assert_eq!(decoder.rows_remaining(), 2);

        // Second strip: only 2 rows (partial)
        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 2);
        assert_eq!(decoder.rows_remaining(), 0);

        // EOF
        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 0);
    }
}
