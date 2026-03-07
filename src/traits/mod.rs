//! Common codec traits.
//!
//! ```text
//! ENCODE:
//!                                  ┌→ Enc (implements Encoder)
//! EncoderConfig → EncodeJob<'a> ──┤
//!                                  └→ FrameEnc (implements FrameEncoder)
//!
//! DECODE:
//!                                  ┌→ Dec (implements Decode)
//! DecoderConfig → DecodeJob<'a> ──┤
//!                                  └→ FrameDec (implements FrameDecode)
//! ```
//!
//! Encoding and decoding are both **type-erased**: encoders accept any pixel
//! format at runtime via [`PixelSlice`] and dispatch internally based on the
//! descriptor. Decoders return native pixels; the caller provides a ranked
//! preference list of [`PixelDescriptor`](crate::PixelDescriptor)s.
//!
//! Color management is explicitly **not** the codec's job. Decoders return
//! native pixels with ICC/CICP metadata. Encoders accept pixels as-is and
//! embed the provided metadata. The caller handles CMS transforms.

mod decoder;
mod decoding;
mod dyn_decoding;
mod dyn_encoding;
mod encoder;
mod encoding;

pub use decoder::{Decode, FrameDecode, StreamingDecode};
pub use decoding::{DecodeJob, DecoderConfig};
pub use dyn_decoding::{
    DynDecodeJob, DynDecoder, DynDecoderConfig, DynFrameDecoder, DynStreamingDecoder,
};
pub use dyn_encoding::{DynEncodeJob, DynEncoder, DynEncoderConfig, DynFrameEncoder};
pub use encoder::{Encoder, FrameEncoder};
pub use encoding::{EncodeJob, EncoderConfig};

use alloc::boxed::Box;

/// Boxed error type for type-erased codec operations.
///
/// Used by [`EncodeJob::dyn_encoder`], [`DecodeJob::dyn_decoder`], and
/// related methods that erase the concrete codec type.
pub type BoxedError = Box<dyn core::error::Error + Send + Sync>;
