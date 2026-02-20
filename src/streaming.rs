//! Pull-based scanline decode for streaming pipelines.
//!
//! [`ScanlineDecoder`] complements the one-shot [`Decoder`](crate::Decoder)
//! with stateful, caller-driven row decoding. The caller pulls rows on demand
//! instead of the codec pushing rows to a callback.
//!
//! Created via [`DecodeJob::scanline_decoder()`](crate::DecodeJob::scanline_decoder).
//! Not every codec supports this — check
//! [`CodecCapabilities::scanline_decode()`](crate::CodecCapabilities::scanline_decode).
//! Codecs that don't support it use [`NeverScanlineDecoder`] and return an
//! error from the factory method.
//!
//! ## Existing row-level capabilities
//!
//! The encode side already has streaming via
//! [`Encoder::push_rows()`](crate::Encoder::push_rows) +
//! [`Encoder::finish()`](crate::Encoder::finish), with
//! [`Encoder::preferred_strip_height()`](crate::Encoder::preferred_strip_height)
//! for optimal strip sizing.
//!
//! The decode side has push-based rows via
//! [`Decoder::decode_rows()`](crate::Decoder::decode_rows), where the codec
//! drives the loop. `ScanlineDecoder` inverts this: the caller drives.
//!
//! ## Example
//!
//! ```text
//! let mut decoder = config.job().scanline_decoder(data)?;
//! let info = decoder.image_info();
//! let desc = decoder.output_descriptor();
//! let strip_h = decoder.preferred_strip_height();
//!
//! let mut buf = PixelBuffer::new(info.width, strip_h, desc);
//! loop {
//!     let rows = decoder.read_rows(buf.as_slice_mut())?;
//!     if rows == 0 { break; }
//!     // process rows...
//! }
//! ```
//!
//! ## Which codecs support this?
//!
//! | Codec | Scanline decode | Notes |
//! |-------|:-:|---|
//! | JPEG  | Yes | MCU-row granularity (8 or 16 rows) |
//! | PNG   | Yes | Row-at-a-time defiltering |
//! | GIF   | No  | Frame-at-once (LZW + palette) |
//! | WebP  | No  | Frame-at-once (VP8/VP8L) |
//! | AVIF  | No  | Frame-at-once (AV1 tiles) |
//! | JXL   | Possible | Group-level streaming (future) |

use core::marker::PhantomData;

use crate::buffer::PixelDescriptor;
use crate::{ImageInfo, PixelSliceMut};

/// Pull-based scanline decoder for streaming pipelines.
///
/// Parses the header upfront, then produces decoded rows on demand.
/// The caller provides a destination buffer and the decoder fills it
/// with the next batch of scanlines.
///
/// Created via [`DecodeJob::scanline_decoder()`](crate::DecodeJob::scanline_decoder).
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
// NeverScanlineDecoder — for codecs that don't support pull-based decode
// ===========================================================================

/// A [`ScanlineDecoder`] that can never be constructed.
///
/// Use as `type ScanlineDecoder = NeverScanlineDecoder<Self::Error>` for
/// codecs that don't support pull-based scanline decoding. The corresponding
/// [`scanline_decoder()`](crate::DecodeJob::scanline_decoder) method should
/// always return `Err(...)`.
///
/// Since this type is uninhabited, all trait method implementations are
/// unreachable — they exist only to satisfy the type system.
pub enum NeverScanlineDecoder<E> {
    #[doc(hidden)]
    _Never(core::convert::Infallible, PhantomData<E>),
}

// Safety: NeverScanlineDecoder is uninhabited, so Send is trivially satisfied.
// PhantomData<E> requires E: Send, which is guaranteed by the Error bound.

impl<E: core::error::Error + Send + Sync + 'static> ScanlineDecoder for NeverScanlineDecoder<E> {
    type Error = E;

    fn image_info(&self) -> &ImageInfo {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }

    fn output_descriptor(&self) -> PixelDescriptor {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }

    fn preferred_strip_height(&self) -> u32 {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }

    fn rows_remaining(&self) -> u32 {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }

    fn read_rows(&mut self, _dst: PixelSliceMut<'_>) -> Result<u32, Self::Error> {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }

    fn seek_to_row(&mut self, _y: u32) -> Result<(), Self::Error> {
        match self {
            NeverScanlineDecoder::_Never(x, _) => match *x {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use crate::{ImageFormat, ImageInfo, PixelDescriptor};

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
            8
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

    #[test]
    fn scanline_decoder_basic() {
        let mut decoder = MockScanlineDecoder::new(16, 32);
        assert_eq!(decoder.image_info().width, 16);
        assert_eq!(decoder.image_info().height, 32);
        assert_eq!(decoder.rows_remaining(), 32);
        assert_eq!(decoder.preferred_strip_height(), 8);
        assert!(!decoder.supports_seek());
        assert!(decoder.seek_to_row(0).is_err());

        let desc = decoder.output_descriptor();
        let bpp = desc.bytes_per_pixel() as usize;
        let stride = 16 * bpp;
        let mut buf = vec![0u8; stride * 8];

        let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 8);
        assert_eq!(decoder.rows_remaining(), 24);

        let mut total_rows = rows;
        while decoder.rows_remaining() > 0 {
            let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
                .expect("valid buffer");
            let rows = decoder.read_rows(slice).unwrap();
            assert!(rows > 0);
            total_rows += rows;
        }
        assert_eq!(total_rows, 32);

        // EOF
        let slice = PixelSliceMut::new(&mut buf, 16, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn scanline_decoder_partial_last_strip() {
        let mut decoder = MockScanlineDecoder::new(4, 10);
        let desc = decoder.output_descriptor();
        let bpp = desc.bytes_per_pixel() as usize;
        let stride = 4 * bpp;
        let mut buf = vec![0u8; stride * 8];

        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 8);
        assert_eq!(decoder.rows_remaining(), 2);

        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 2);
        assert_eq!(decoder.rows_remaining(), 0);

        let slice = PixelSliceMut::new(&mut buf, 4, 8, stride, desc)
            .expect("valid buffer");
        let rows = decoder.read_rows(slice).unwrap();
        assert_eq!(rows, 0);
    }
}
