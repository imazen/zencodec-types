//! Common codec traits.
//!
//! These traits define the execution interface for image codecs. Configuration
//! (quality, effort, lossless, etc.) lives on each codec's concrete types —
//! the traits handle execution, metadata, cancellation, and resource limits.
//!
//! Individual codecs implement these traits on their config types.
//! Format-specific settings live on the concrete types, not on the traits.

use imgref::ImgRef;
use rgb::alt::BGRA;
use rgb::{Gray, Rgb, Rgba};

use imgref::ImgRefMut;

use crate::{
    CodecCapabilities, DecodeOutput, EncodeOutput, ImageInfo, ImageMetadata, ResourceLimits, Stop,
};

/// Common interface for encode configurations.
///
/// Implemented by each codec's config type (e.g. `zenjpeg::EncoderConfig`).
/// Config types are reusable (`Clone`) and have no lifetimes — they can be
/// stored in structs and shared across threads.
///
/// Format-specific settings (quality, effort, lossless mode) are set on the
/// concrete config type before it enters the trait interface. The trait handles
/// only universal concerns: resource limits and job creation.
///
/// The `job()` method creates a per-operation [`EncodingJob`] that can borrow
/// temporary data (stop tokens, metadata).
pub trait Encoding: Sized + Clone + Send + Sync {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Per-operation job type, created by [`job()`](Encoding::job).
    type Job<'a>: EncodingJob<'a, Error = Self::Error>
    where
        Self: 'a;

    /// Codec capabilities (metadata support, cancellation, etc.).
    ///
    /// Returns a static reference describing what this codec supports.
    /// Use this to check before calling methods that may be no-ops.
    fn capabilities() -> &'static CodecCapabilities;

    /// Apply resource limits.
    ///
    /// Codecs enforce the limits they support (pixel count, memory, output size).
    /// Check [`capabilities()`](Encoding::capabilities) to see which limits are enforced.
    fn with_limits(self, limits: ResourceLimits) -> Self;

    /// Create a per-operation job for this config.
    ///
    /// The job borrows the config and can accept temporary references
    /// (stop tokens, metadata) before executing.
    fn job(&self) -> Self::Job<'_>;

    /// Convenience: encode RGB8 with default job settings.
    fn encode_rgb8(&self, img: ImgRef<'_, Rgb<u8>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgb8(img)
    }

    /// Convenience: encode RGBA8 with default job settings.
    fn encode_rgba8(&self, img: ImgRef<'_, Rgba<u8>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgba8(img)
    }

    /// Convenience: encode Gray8 with default job settings.
    fn encode_gray8(&self, img: ImgRef<'_, Gray<u8>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray8(img)
    }

    /// Convenience: encode BGRA8 with default job settings.
    fn encode_bgra8(&self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_bgra8(img)
    }

    /// Convenience: encode BGRX8 (opaque BGRA, padding byte ignored) with default job settings.
    fn encode_bgrx8(&self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_bgrx8(img)
    }
}

/// Per-operation encode job.
///
/// Created by [`Encoding::job()`]. Borrows temporary data (stop token,
/// metadata) and is consumed by terminal encode methods.
///
/// Every codec must accept a stop token and metadata. The codec embeds
/// whatever metadata the format supports and periodically checks the
/// stop token for cooperative cancellation.
///
/// Check [`Encoding::capabilities()`] to see which metadata types and
/// cancellation are actually supported.
pub trait EncodingJob<'a>: Sized {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Set cooperative cancellation token.
    ///
    /// The codec periodically calls `stop.check()` and returns an error
    /// if the operation should be cancelled. No-op if the codec doesn't
    /// support cancellation (check [`capabilities().encode_cancel()`](CodecCapabilities::encode_cancel)).
    fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Set all metadata (ICC, EXIF, XMP) from an [`ImageMetadata`].
    ///
    /// The codec embeds whatever metadata the format supports. Metadata
    /// types not supported by the format are silently skipped — check
    /// [`capabilities()`](Encoding::capabilities) to see what's supported.
    fn with_metadata(self, meta: &'a ImageMetadata<'a>) -> Self;

    /// Override resource limits for this operation.
    fn with_limits(self, limits: ResourceLimits) -> Self;

    /// Encode RGB8 pixels.
    fn encode_rgb8(self, img: ImgRef<'_, Rgb<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode RGBA8 pixels.
    fn encode_rgba8(self, img: ImgRef<'_, Rgba<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode grayscale 8-bit pixels.
    fn encode_gray8(self, img: ImgRef<'_, Gray<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode BGRA8 pixels.
    ///
    /// Default implementation swizzles to RGBA8 and delegates to [`encode_rgba8`](EncodingJob::encode_rgba8).
    /// Codecs that support BGRA natively should override this.
    fn encode_bgra8(self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error> {
        let (buf, w, h) = img.to_contiguous_buf();
        let rgba: alloc::vec::Vec<Rgba<u8>> = buf
            .iter()
            .map(|p| Rgba {
                r: p.r,
                g: p.g,
                b: p.b,
                a: p.a,
            })
            .collect();
        let rgba_img = imgref::ImgVec::new(rgba, w, h);
        self.encode_rgba8(rgba_img.as_ref())
    }

    /// Encode BGRX8 pixels (opaque BGRA — padding byte is ignored).
    ///
    /// Default implementation swizzles to RGB8 and delegates to [`encode_rgb8`](EncodingJob::encode_rgb8).
    /// Codecs that support BGRX natively should override this.
    fn encode_bgrx8(self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error> {
        let (buf, w, h) = img.to_contiguous_buf();
        let rgb: alloc::vec::Vec<Rgb<u8>> = buf
            .iter()
            .map(|p| Rgb {
                r: p.r,
                g: p.g,
                b: p.b,
            })
            .collect();
        let rgb_img = imgref::ImgVec::new(rgb, w, h);
        self.encode_rgb8(rgb_img.as_ref())
    }
}

/// Common interface for decode configurations.
///
/// Implemented by each codec's config type (e.g. `zenjpeg::DecodeConfig`).
/// Config types are reusable (`Clone`) and have no lifetimes.
///
/// Format-specific decode settings live on the concrete config type.
/// The trait handles resource limits, job creation, and probing.
pub trait Decoding: Sized + Clone + Send + Sync {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Per-operation job type, created by [`job()`](Decoding::job).
    type Job<'a>: DecodingJob<'a, Error = Self::Error>
    where
        Self: 'a;

    /// Codec capabilities (metadata support, cancellation, probe cost, etc.).
    ///
    /// Returns a static reference describing what this codec supports.
    fn capabilities() -> &'static CodecCapabilities;

    /// Apply resource limits.
    ///
    /// Codecs enforce the limits they support (pixel count, memory, dimensions,
    /// file size). Check [`capabilities()`](Decoding::capabilities) to see which
    /// limits are enforced.
    fn with_limits(self, limits: ResourceLimits) -> Self;

    /// Create a per-operation job for this config.
    fn job(&self) -> Self::Job<'_>;

    /// Probe image metadata cheaply (header parse only).
    ///
    /// This MUST be cheap — O(header), not O(pixels). Parses container
    /// headers to extract dimensions, format, and basic metadata. May not
    /// return frame counts or other data requiring a full parse.
    ///
    /// Use [`probe_full`](Decoding::probe_full) when you need complete
    /// metadata including frame counts.
    fn probe_header(&self, data: &[u8]) -> Result<ImageInfo, Self::Error>;

    /// Probe image metadata with a full parse.
    ///
    /// May be expensive (e.g. parsing all GIF frames to count them, or
    /// decoding AVIF container metadata). Returns complete metadata
    /// including frame counts.
    ///
    /// Default: delegates to [`probe_header`](Decoding::probe_header).
    /// Codecs that need a full parse for complete metadata should override.
    fn probe_full(&self, data: &[u8]) -> Result<ImageInfo, Self::Error> {
        self.probe_header(data)
    }

    /// Convenience: decode with default job settings.
    fn decode(&self, data: &[u8]) -> Result<DecodeOutput, Self::Error> {
        self.job().decode(data)
    }

    /// Convenience: decode into a caller-provided RGB8 buffer.
    fn decode_into_rgb8(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgb8(data, dst)
    }

    /// Convenience: decode into a caller-provided RGBA8 buffer.
    fn decode_into_rgba8(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgba8(data, dst)
    }

    /// Convenience: decode into a caller-provided Gray8 buffer.
    fn decode_into_gray8(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_gray8(data, dst)
    }

    /// Compute output dimensions/info for this data given current config.
    ///
    /// Unlike [`probe_header()`](Decoding::probe_header) which returns stored
    /// file dimensions, this applies config transforms (scaling, orientation)
    /// to predict actual decode output. Use this to allocate buffers for
    /// `decode_into_*` methods.
    ///
    /// Default: delegates to `probe_header()` (correct when config doesn't transform dims).
    fn decode_info(&self, data: &[u8]) -> Result<ImageInfo, Self::Error> {
        self.probe_header(data)
    }
}

/// Per-operation decode job.
///
/// Created by [`Decoding::job()`]. Borrows temporary data (stop token)
/// and is consumed by terminal decode methods.
pub trait DecodingJob<'a>: Sized {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Set cooperative cancellation token.
    ///
    /// No-op if the codec doesn't support decode cancellation
    /// (check [`capabilities().decode_cancel()`](CodecCapabilities::decode_cancel)).
    fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Override resource limits for this operation.
    fn with_limits(self, limits: ResourceLimits) -> Self;

    /// Decode image data to pixels.
    fn decode(self, data: &[u8]) -> Result<DecodeOutput, Self::Error>;

    /// Decode directly into a caller-provided RGB8 buffer (zero-copy path).
    ///
    /// The buffer must have dimensions matching [`Decoding::decode_info()`] results
    /// (use [`display_width()`](ImageInfo::display_width) /
    /// [`display_height()`](ImageInfo::display_height) if orientation may be applied).
    ///
    /// Returns [`ImageInfo`] with metadata from the decoded image.
    ///
    /// Default implementation calls [`decode()`](DecodingJob::decode), converts to
    /// RGB8, and copies. Codecs that can decode directly into the caller's buffer
    /// should override for true zero-copy.
    fn decode_into_rgb8(
        self,
        data: &[u8],
        mut dst: ImgRefMut<'_, Rgb<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        let output = self.decode(data)?;
        let info = output.info().clone();
        let src = output.into_rgb8();
        for (src_row, dst_row) in src.as_ref().rows().zip(dst.rows_mut()) {
            let n = src_row.len().min(dst_row.len());
            dst_row[..n].copy_from_slice(&src_row[..n]);
        }
        Ok(info)
    }

    /// Decode directly into a caller-provided RGBA8 buffer (zero-copy path).
    ///
    /// Same contract as [`decode_into_rgb8`](DecodingJob::decode_into_rgb8).
    fn decode_into_rgba8(
        self,
        data: &[u8],
        mut dst: ImgRefMut<'_, Rgba<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        let output = self.decode(data)?;
        let info = output.info().clone();
        let src = output.into_rgba8();
        for (src_row, dst_row) in src.as_ref().rows().zip(dst.rows_mut()) {
            let n = src_row.len().min(dst_row.len());
            dst_row[..n].copy_from_slice(&src_row[..n]);
        }
        Ok(info)
    }

    /// Decode directly into a caller-provided Gray8 buffer (zero-copy path).
    ///
    /// Same contract as [`decode_into_rgb8`](DecodingJob::decode_into_rgb8).
    fn decode_into_gray8(
        self,
        data: &[u8],
        mut dst: ImgRefMut<'_, Gray<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        let output = self.decode(data)?;
        let info = output.info().clone();
        let src = output.into_gray8();
        for (src_row, dst_row) in src.as_ref().rows().zip(dst.rows_mut()) {
            let n = src_row.len().min(dst_row.len());
            dst_row[..n].copy_from_slice(&src_row[..n]);
        }
        Ok(info)
    }
}
