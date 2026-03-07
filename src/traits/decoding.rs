//! Decoder configuration and decode jobs.

use alloc::borrow::Cow;
use alloc::boxed::Box;

use crate::format::ImageFormat;
use crate::orientation::OrientationHint;
use crate::{DecodeCapabilities, ImageInfo, OutputInfo, ResourceLimits};
use enough::Stop;
use zenpixels::PixelDescriptor;

use super::BoxedError;
use super::decoder::{Decode, FrameDecode, StreamingDecode};
use super::dyn_decoding::{
    DecoderShim, DynDecoder, DynFrameDecoder, DynStreamingDecoder, FrameDecoderShim,
    StreamingDecoderShim,
};

// ===========================================================================
// Decoder configuration
// ===========================================================================

/// Reusable decoder configuration.
///
/// Implemented by each codec's config type. Config types are `Clone + Send +
/// Sync` with no lifetimes.
///
/// Probing lives on [`DecodeJob`], not here, because probing needs limits
/// and cancellation context.
pub trait DecoderConfig: Clone + Send + Sync {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Per-operation job type.
    type Job<'a>: DecodeJob<'a, Error = Self::Error>
    where
        Self: 'a;

    /// The image format this decoder handles.
    fn format() -> ImageFormat;

    /// Pixel formats this decoder can produce natively.
    ///
    /// Every descriptor is a guarantee: the decoder can produce this format
    /// without lossy conversion. Must not be empty.
    fn supported_descriptors() -> &'static [PixelDescriptor];

    /// Decoder capabilities (metadata support, cancellation, etc.).
    ///
    /// Returns a static reference describing what this decoder supports.
    fn capabilities() -> &'static DecodeCapabilities {
        &DecodeCapabilities::EMPTY
    }

    /// Create a per-operation job.
    fn job(&self) -> Self::Job<'_>;
}

// ===========================================================================
// Decode job
// ===========================================================================

/// Per-operation decode job.
///
/// Created by [`DecoderConfig::job()`]. Holds limits, cancellation, and
/// decode hints. Probing lives here because it needs the limits/stop context.
///
/// # Decode hints
///
/// Hints let the caller request spatial transforms (crop, scale, orientation)
/// that the decoder may apply during decode. The decoder is free to ignore
/// any hint. Call [`output_info()`](DecodeJob::output_info) after setting
/// hints to learn what the decoder will actually produce.
pub trait DecodeJob<'a>: Sized {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Single-image decoder type.
    type Dec: Decode<Error = Self::Error>;

    /// Streaming decoder type.
    ///
    /// Implements [`StreamingDecode`] for batch/scanline-level decode.
    /// Set to `()` if the codec does not support streaming decode.
    type StreamDec: StreamingDecode<Error = Self::Error>;

    /// Animation decoder type.
    ///
    /// Must be `'static` — frame decoders own their data (typically by
    /// copying the input slice at construction time). This lets callers
    /// drop the input buffer while still iterating frames.
    type FrameDec: FrameDecode<Error = Self::Error> + 'static;

    /// Set cooperative cancellation token.
    fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Override resource limits for this operation.
    fn with_limits(self, limits: ResourceLimits) -> Self;

    /// Set decode security policy (controls metadata extraction, parsing strictness, etc.).
    ///
    /// Default no-op. Codecs that support policy check the flags in
    /// [`DecodePolicy`](crate::decode::DecodePolicy) to decide what to extract and accept.
    fn with_policy(self, _policy: crate::DecodePolicy) -> Self {
        self
    }

    // --- Probing (needs limits + stop context) ---

    /// Probe image metadata cheaply (header parse only).
    ///
    /// O(header), not O(pixels). Parses container headers to extract
    /// dimensions, format, and basic metadata. May not return frame
    /// counts or data requiring a full parse.
    fn probe(&self, data: &[u8]) -> Result<ImageInfo, Self::Error>;

    /// Probe image metadata with a full parse.
    ///
    /// May be expensive (e.g., parsing all GIF frames to count them).
    /// Returns complete metadata including frame counts.
    ///
    /// Default: delegates to [`probe()`](DecodeJob::probe).
    fn probe_full(&self, data: &[u8]) -> Result<ImageInfo, Self::Error> {
        self.probe(data)
    }

    // --- Decode hints (optional, decoder may ignore) ---

    /// Hint: crop to this region in source coordinates.
    ///
    /// The decoder may adjust for block alignment (JPEG MCU boundaries).
    fn with_crop_hint(self, _x: u32, _y: u32, _width: u32, _height: u32) -> Self {
        self
    }

    /// Hint: target output dimensions for prescaling.
    ///
    /// Some codecs decode at reduced resolution cheaply (JPEG 1/2/4/8).
    fn with_scale_hint(self, _max_width: u32, _max_height: u32) -> Self {
        self
    }

    /// Set orientation handling strategy.
    ///
    /// See [`OrientationHint`] for the available strategies.
    /// Default: [`OrientationHint::Preserve`].
    fn with_orientation(self, _hint: OrientationHint) -> Self {
        self
    }

    /// Hint: seek to a specific frame before decoding.
    ///
    /// For animation formats that support random access (e.g., AVIF),
    /// this lets the decoder skip directly to the requested frame.
    /// The decoder may ignore this hint if random access is not supported.
    ///
    /// Only meaningful before [`frame_decoder()`](DecodeJob::frame_decoder).
    fn with_frame_index(self, _index: u32) -> Self {
        self
    }

    // --- Output prediction ---

    /// Predict what the decoder will produce given current hints.
    ///
    /// Returns dimensions, pixel format, and which hints were honored.
    /// Call after setting hints, before creating a decoder.
    fn output_info(&self, data: &[u8]) -> Result<OutputInfo, Self::Error>;

    // --- Executor creation ---
    //
    // All executors bind `data` here so the DecodeJob is the single
    // place where input is provided. This keeps Decode/StreamingDecode/
    // FrameDecode free of data parameters, and prepares for future
    // IO-read sources (the job can bind a reader instead of a slice).
    //
    // Consistent parameter order: data, [sink], preferred.

    /// Create a one-shot decoder bound to `data`.
    ///
    /// The decoder stores the [`Cow`] and borrows from it via [`Deref`].
    /// Pass `Cow::Borrowed(&slice)` for zero-copy slice access, or
    /// `Cow::Owned(vec)` to donate a buffer (avoids a copy in codecs
    /// that need owned data internally).
    ///
    /// `preferred` is a ranked list of desired output formats. The decoder
    /// picks the first it can produce without lossy conversion. Pass `&[]`
    /// for the decoder's native format.
    fn decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Self::Dec, Self::Error>;

    /// Decode directly into a caller-owned sink (push model).
    ///
    /// Decodes and pushes strips into `sink` via
    /// [`crate::DecodeRowSink::demand`]. Returns [`OutputInfo`] describing
    /// what was produced (pixels went into the sink, not a return value).
    ///
    /// `preferred` is a ranked list of desired output formats.
    ///
    /// Default implementation creates a [`decoder()`](DecodeJob::decoder),
    /// calls [`Decode::decode()`], then copies the result into the sink
    /// strip by strip. Codecs with native row streaming should override
    /// this for zero-copy.
    fn push_decoder(
        self,
        data: Cow<'a, [u8]>,
        sink: &mut dyn crate::DecodeRowSink,
        preferred: &[PixelDescriptor],
    ) -> Result<OutputInfo, Self::Error> {
        let dec = self.decoder(data, preferred)?;
        let output = dec.decode()?;
        let ps = output.pixels();
        let desc = ps.descriptor();
        let w = ps.width();
        let h = ps.rows();

        // Push all rows into the sink as a single strip
        let mut dst = sink.demand(0, h, w, desc);
        for row in 0..h {
            dst.row_mut(row).copy_from_slice(ps.row(row));
        }

        let info = output.info();
        Ok(OutputInfo::full_decode(info.width, info.height, desc))
    }

    /// Create a streaming decoder that yields scanline batches.
    ///
    /// Returns an error if the codec does not support streaming decode.
    ///
    /// `preferred` is a ranked list of desired output formats.
    ///
    /// See [`StreamingDecode`] for the batch pull API.
    fn streaming_decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Self::StreamDec, Self::Error>;

    /// Create a frame-by-frame animation decoder.
    ///
    /// The decoder calls [`Cow::into_owned()`] to take ownership of the
    /// data (required because `FrameDec: 'static`). When the caller
    /// passes `Cow::Owned(vec)`, this is a free move with no copy.
    ///
    /// `preferred` is a ranked list of desired output formats.
    fn frame_decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Self::FrameDec, Self::Error>;

    // --- Type-erased convenience methods ---

    /// Create a type-erased one-shot decoder.
    ///
    /// Returns a boxed closure that decodes to owned pixels. All hints
    /// and preferences are bound before this call.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let decode = config.job()
    ///     .with_scale_hint(800, 600)
    ///     .dyn_decoder(data, &[PixelDescriptor::rgb8()])?;
    ///
    /// let output: DecodeOutput = decode.decode()?;
    /// ```
    fn dyn_decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Box<dyn DynDecoder + 'a>, BoxedError>
    where
        Self: 'a,
    {
        let dec = self
            .decoder(data, preferred)
            .map_err(|e| Box::new(e) as BoxedError)?;
        Ok(Box::new(DecoderShim(dec)))
    }

    /// Create a type-erased frame-by-frame decoder.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut dec = config.job()
    ///     .dyn_frame_decoder(data, &[])?;
    ///
    /// while let Some(frame) = dec.next_frame()? {
    ///     // process frame
    /// }
    /// ```
    fn dyn_frame_decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Box<dyn DynFrameDecoder>, BoxedError>
    where
        Self: 'a,
    {
        let dec = self
            .frame_decoder(data, preferred)
            .map_err(|e| Box::new(e) as BoxedError)?;
        Ok(Box::new(FrameDecoderShim(dec)))
    }

    /// Create a type-erased streaming decoder.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut dec = config.job()
    ///     .dyn_streaming_decoder(data, &[])?;
    ///
    /// while let Some((y, strip)) = dec.next_batch()? {
    ///     // process strip
    /// }
    /// ```
    fn dyn_streaming_decoder(
        self,
        data: Cow<'a, [u8]>,
        preferred: &[PixelDescriptor],
    ) -> Result<Box<dyn DynStreamingDecoder + 'a>, BoxedError>
    where
        Self: 'a,
    {
        let dec = self
            .streaming_decoder(data, preferred)
            .map_err(|e| Box::new(e) as BoxedError)?;
        Ok(Box::new(StreamingDecoderShim(dec)))
    }
}
