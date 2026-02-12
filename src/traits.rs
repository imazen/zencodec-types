//! Common codec traits.
//!
//! Individual codecs implement these traits on their config types.
//! Format-specific methods live on the concrete types, not on the traits.

use imgref::ImgRef;
use rgb::alt::BGRA;
use rgb::{Gray, Rgb, Rgba};

use crate::{DecodeOutput, EncodeOutput, ImageInfo, ImageMetadata, Stop};

/// Common interface for encode configurations.
///
/// Implemented by each codec's config type (e.g. `zenjpeg::EncodeConfig`).
/// Config types are reusable (`Clone`) and have no lifetimes — they can be
/// stored in structs and shared across threads.
///
/// The `job()` method creates a per-operation [`EncodingJob`] that can borrow
/// temporary data (stop tokens, metadata).
pub trait Encoding: Sized + Clone {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Per-operation job type, created by [`job()`](Encoding::job).
    type Job<'a>: EncodingJob<'a, Error = Self::Error>
    where
        Self: 'a;

    /// Set encode quality (0.0–100.0, codec-mapped).
    fn with_quality(self, quality: f32) -> Self;

    /// Set encode effort / speed tradeoff (0–10, codec-mapped).
    fn with_effort(self, effort: u32) -> Self;

    /// Request lossless encoding (not all codecs support this).
    fn with_lossless(self, lossless: bool) -> Self;

    /// Set alpha channel quality (0.0–100.0, codec-mapped).
    fn with_alpha_quality(self, quality: f32) -> Self;

    /// Limit maximum pixel count (width * height).
    fn with_limit_pixels(self, max: u64) -> Self;

    /// Limit maximum memory usage in bytes.
    fn with_limit_memory(self, bytes: u64) -> Self;

    /// Limit maximum output size in bytes.
    fn with_limit_output(self, bytes: u64) -> Self;

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
/// metadata) and is consumed by terminal methods.
pub trait EncodingJob<'a>: Sized {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Set cooperative cancellation token.
    fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Set all metadata (ICC, EXIF, XMP) from an [`ImageMetadata`].
    fn with_metadata(self, meta: &'a ImageMetadata<'a>) -> Self;

    /// Set ICC color profile.
    fn with_icc(self, icc: &'a [u8]) -> Self;

    /// Set EXIF metadata.
    fn with_exif(self, exif: &'a [u8]) -> Self;

    /// Set XMP metadata.
    fn with_xmp(self, xmp: &'a [u8]) -> Self;

    /// Override config pixel limit for this operation.
    fn with_limit_pixels(self, max: u64) -> Self;

    /// Override config memory limit for this operation.
    fn with_limit_memory(self, bytes: u64) -> Self;

    /// Encode RGB8 pixels.
    fn encode_rgb8(self, img: ImgRef<'_, Rgb<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode RGBA8 pixels.
    fn encode_rgba8(self, img: ImgRef<'_, Rgba<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode grayscale 8-bit pixels.
    fn encode_gray8(self, img: ImgRef<'_, Gray<u8>>) -> Result<EncodeOutput, Self::Error>;

    /// Encode BGRA8 pixels.
    ///
    /// Default implementation swizzles to RGBA8 and delegates to [`encode_rgba8`].
    /// Codecs that support BGRA natively (e.g. zenjpeg) should override this
    /// to avoid the intermediate conversion.
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
    /// Default implementation swizzles to RGB8 and delegates to [`encode_rgb8`].
    /// Codecs that support BGRX natively (e.g. zenjpeg) should override this
    /// to avoid the intermediate conversion.
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
pub trait Decoding: Sized + Clone {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Per-operation job type, created by [`job()`](Decoding::job).
    type Job<'a>: DecodingJob<'a, Error = Self::Error>
    where
        Self: 'a;

    /// Limit maximum pixel count (width * height).
    fn with_limit_pixels(self, max: u64) -> Self;

    /// Limit maximum memory usage in bytes.
    fn with_limit_memory(self, bytes: u64) -> Self;

    /// Limit maximum image dimensions.
    fn with_limit_dimensions(self, width: u32, height: u32) -> Self;

    /// Limit maximum input file size in bytes.
    fn with_limit_file_size(self, bytes: u64) -> Self;

    /// Create a per-operation job for this config.
    fn job(&self) -> Self::Job<'_>;

    /// Convenience: decode with default job settings.
    fn decode(&self, data: &[u8]) -> Result<DecodeOutput, Self::Error> {
        self.job().decode(data)
    }

    /// Convenience: probe metadata with default job settings.
    fn probe(&self, data: &[u8]) -> Result<ImageInfo, Self::Error>;
}

/// Per-operation decode job.
///
/// Created by [`Decoding::job()`]. Borrows temporary data (stop token)
/// and is consumed by terminal methods.
pub trait DecodingJob<'a>: Sized {
    /// The codec-specific error type.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Set cooperative cancellation token.
    fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Override config pixel limit for this operation.
    fn with_limit_pixels(self, max: u64) -> Self;

    /// Override config memory limit for this operation.
    fn with_limit_memory(self, bytes: u64) -> Self;

    /// Decode image data to pixels.
    fn decode(self, data: &[u8]) -> Result<DecodeOutput, Self::Error>;
}
