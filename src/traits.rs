//! Common codec traits.
//!
//! These traits define the execution interface for image codecs. Configuration
//! (quality, effort, lossless, etc.) lives on each codec's concrete types —
//! the traits handle execution, metadata, cancellation, and resource limits.
//!
//! Individual codecs implement these traits on their config types.
//! Format-specific settings live on the concrete types, not on the traits.

use alloc::vec::Vec;
use imgref::ImgRef;
use rgb::alt::BGRA;

use crate::pixel::GrayAlpha;
use rgb::{Gray, Rgb, Rgba};

use imgref::ImgRefMut;

use crate::output::EncodeFrame;
use crate::{
    CodecCapabilities, DecodeFrame, DecodeOutput, EncodeOutput, ImageInfo, ImageMetadata,
    ResourceLimits, Stop,
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

    /// Convenience: encode linear RGB f32 with default job settings.
    fn encode_rgb_f32(&self, img: ImgRef<'_, Rgb<f32>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgb_f32(img)
    }

    /// Convenience: encode linear RGBA f32 with default job settings.
    fn encode_rgba_f32(&self, img: ImgRef<'_, Rgba<f32>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgba_f32(img)
    }

    /// Convenience: encode linear grayscale f32 with default job settings.
    fn encode_gray_f32(&self, img: ImgRef<'_, Gray<f32>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray_f32(img)
    }

    /// Convenience: encode RGB16 with default job settings.
    fn encode_rgb16(&self, img: ImgRef<'_, Rgb<u16>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgb16(img)
    }

    /// Convenience: encode RGBA16 with default job settings.
    fn encode_rgba16(&self, img: ImgRef<'_, Rgba<u16>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_rgba16(img)
    }

    /// Convenience: encode Gray16 with default job settings.
    fn encode_gray16(&self, img: ImgRef<'_, Gray<u16>>) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray16(img)
    }

    /// Convenience: encode GrayAlpha8 with default job settings.
    fn encode_gray_alpha8(
        &self,
        img: ImgRef<'_, GrayAlpha<u8>>,
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray_alpha8(img)
    }

    /// Convenience: encode GrayAlpha16 with default job settings.
    fn encode_gray_alpha16(
        &self,
        img: ImgRef<'_, GrayAlpha<u16>>,
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray_alpha16(img)
    }

    /// Convenience: encode linear GrayAlpha f32 with default job settings.
    fn encode_gray_alpha_f32(
        &self,
        img: ImgRef<'_, GrayAlpha<f32>>,
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_gray_alpha_f32(img)
    }

    /// Convenience: encode RGB8 animation with default job settings.
    fn encode_animation_rgb8(
        &self,
        frames: &[EncodeFrame<'_, Rgb<u8>>],
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_animation_rgb8(frames)
    }

    /// Convenience: encode RGBA8 animation with default job settings.
    fn encode_animation_rgba8(
        &self,
        frames: &[EncodeFrame<'_, Rgba<u8>>],
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_animation_rgba8(frames)
    }

    /// Convenience: encode 16-bit RGB animation with default job settings.
    fn encode_animation_rgb16(
        &self,
        frames: &[EncodeFrame<'_, Rgb<u16>>],
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_animation_rgb16(frames)
    }

    /// Convenience: encode 16-bit RGBA animation with default job settings.
    fn encode_animation_rgba16(
        &self,
        frames: &[EncodeFrame<'_, Rgba<u16>>],
    ) -> Result<EncodeOutput, Self::Error> {
        self.job().encode_animation_rgba16(frames)
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
    fn encode_bgra8(self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode BGRX8 pixels (opaque BGRA — padding byte is ignored).
    fn encode_bgrx8(self, img: ImgRef<'_, BGRA<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode linear RGB f32 pixels.
    ///
    /// Input is expected in linear light (not sRGB gamma). Codecs that store
    /// sRGB should convert using the [`linear_srgb`](https://crates.io/crates/linear_srgb) crate.
    /// Codecs with native f32 support (JXL, PFM) can encode directly.
    fn encode_rgb_f32(self, img: ImgRef<'_, Rgb<f32>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode linear RGBA f32 pixels.
    ///
    /// Input is expected in linear light. See [`encode_rgb_f32`](EncodingJob::encode_rgb_f32).
    fn encode_rgba_f32(self, img: ImgRef<'_, Rgba<f32>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode linear grayscale f32 pixels.
    ///
    /// Input is expected in linear light. See [`encode_rgb_f32`](EncodingJob::encode_rgb_f32).
    fn encode_gray_f32(self, img: ImgRef<'_, Gray<f32>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode 16-bit RGB pixels.
    ///
    /// Codecs without native 16-bit support should dither or truncate to their
    /// native bit depth. Check [`capabilities().native_16bit()`](CodecCapabilities::native_16bit).
    fn encode_rgb16(self, img: ImgRef<'_, Rgb<u16>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode 16-bit RGBA pixels.
    fn encode_rgba16(self, img: ImgRef<'_, Rgba<u16>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode 16-bit grayscale pixels.
    fn encode_gray16(self, img: ImgRef<'_, Gray<u16>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode 8-bit grayscale + alpha pixels.
    fn encode_gray_alpha8(
        self,
        img: ImgRef<'_, GrayAlpha<u8>>,
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode 16-bit grayscale + alpha pixels.
    fn encode_gray_alpha16(
        self,
        img: ImgRef<'_, GrayAlpha<u16>>,
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode linear grayscale + alpha f32 pixels.
    ///
    /// Input is expected in linear light.
    fn encode_gray_alpha_f32(
        self,
        img: ImgRef<'_, GrayAlpha<f32>>,
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode an animation as a sequence of RGB8 frames.
    ///
    /// Codecs that don't support animation should return an error.
    /// Check [`capabilities().encode_animation()`](CodecCapabilities::encode_animation).
    fn encode_animation_rgb8(
        self,
        frames: &[EncodeFrame<'_, Rgb<u8>>],
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode an animation as a sequence of RGBA8 frames.
    ///
    /// Codecs that don't support animation should return an error.
    fn encode_animation_rgba8(
        self,
        frames: &[EncodeFrame<'_, Rgba<u8>>],
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode an animation as a sequence of 16-bit RGB frames.
    ///
    /// Codecs that don't support animation should return an error.
    /// Codecs without native 16-bit support should dither or truncate.
    fn encode_animation_rgb16(
        self,
        frames: &[EncodeFrame<'_, Rgb<u16>>],
    ) -> Result<EncodeOutput, Self::Error>;

    /// Encode an animation as a sequence of 16-bit RGBA frames.
    ///
    /// Codecs that don't support animation should return an error.
    /// Codecs without native 16-bit support should dither or truncate.
    fn encode_animation_rgba16(
        self,
        frames: &[EncodeFrame<'_, Rgba<u16>>],
    ) -> Result<EncodeOutput, Self::Error>;
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

    /// Convenience: decode into a caller-provided BGRA8 buffer.
    fn decode_into_bgra8(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, BGRA<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_bgra8(data, dst)
    }

    /// Convenience: decode into a caller-provided BGRX8 buffer (alpha byte set to 255).
    fn decode_into_bgrx8(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, BGRA<u8>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_bgrx8(data, dst)
    }

    /// Convenience: decode into a caller-provided linear RGB f32 buffer.
    fn decode_into_rgb_f32(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<f32>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgb_f32(data, dst)
    }

    /// Convenience: decode into a caller-provided linear RGBA f32 buffer.
    fn decode_into_rgba_f32(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<f32>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgba_f32(data, dst)
    }

    /// Convenience: decode into a caller-provided linear grayscale f32 buffer.
    fn decode_into_gray_f32(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<f32>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_gray_f32(data, dst)
    }

    /// Convenience: decode into a caller-provided 16-bit RGB buffer.
    fn decode_into_rgb16(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<u16>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgb16(data, dst)
    }

    /// Convenience: decode into a caller-provided 16-bit RGBA buffer.
    fn decode_into_rgba16(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<u16>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_rgba16(data, dst)
    }

    /// Convenience: decode into a caller-provided 16-bit grayscale buffer.
    fn decode_into_gray16(
        &self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<u16>>,
    ) -> Result<ImageInfo, Self::Error> {
        self.job().decode_into_gray16(data, dst)
    }

    /// Convenience: decode all animation frames with default job settings.
    fn decode_animation(&self, data: &[u8]) -> Result<Vec<DecodeFrame>, Self::Error> {
        self.job().decode_animation(data)
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

    /// Decode directly into a caller-provided RGB8 buffer.
    ///
    /// The buffer must have dimensions matching [`Decoding::decode_info()`] results.
    /// Returns [`ImageInfo`] with metadata from the decoded image.
    fn decode_into_rgb8(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<u8>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided RGBA8 buffer.
    fn decode_into_rgba8(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<u8>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided Gray8 buffer.
    fn decode_into_gray8(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<u8>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided BGRA8 buffer.
    fn decode_into_bgra8(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, BGRA<u8>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided BGRX8 buffer (alpha byte set to 255).
    fn decode_into_bgrx8(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, BGRA<u8>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided linear RGB f32 buffer.
    ///
    /// Output is in linear light (not sRGB gamma). Codecs that store sRGB
    /// should convert using the [`linear_srgb`](https://crates.io/crates/linear_srgb) crate.
    /// Codecs with native f32 support (JXL, PFM) can decode directly.
    fn decode_into_rgb_f32(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<f32>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided linear RGBA f32 buffer.
    ///
    /// Output is in linear light. See [`decode_into_rgb_f32`](DecodingJob::decode_into_rgb_f32).
    fn decode_into_rgba_f32(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<f32>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided linear grayscale f32 buffer.
    ///
    /// Output is in linear light. See [`decode_into_rgb_f32`](DecodingJob::decode_into_rgb_f32).
    fn decode_into_gray_f32(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<f32>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided 16-bit RGB buffer.
    ///
    /// Codecs with native 8-bit output should upscale to 16-bit.
    fn decode_into_rgb16(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgb<u16>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided 16-bit RGBA buffer.
    ///
    /// Codecs with native 8-bit output should upscale to 16-bit.
    fn decode_into_rgba16(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Rgba<u16>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode directly into a caller-provided 16-bit grayscale buffer.
    ///
    /// Codecs with native 8-bit output should upscale to 16-bit.
    fn decode_into_gray16(
        self,
        data: &[u8],
        dst: ImgRefMut<'_, Gray<u16>>,
    ) -> Result<ImageInfo, Self::Error>;

    /// Decode all animation frames.
    ///
    /// Returns each frame with its pixel data and duration. For still images,
    /// returns a single frame with duration 0.
    ///
    /// **Note:** All frames are buffered in memory. For large animations
    /// this can require significant memory (e.g. 100 frames at 4K RGBA8
    /// is ~6 GB). A streaming frame iterator API is planned for a future
    /// version.
    ///
    /// Codecs that don't support animation should return the primary image
    /// as a single frame. Check [`capabilities().decode_animation()`](CodecCapabilities::decode_animation).
    fn decode_animation(self, data: &[u8]) -> Result<Vec<DecodeFrame>, Self::Error>;
}
