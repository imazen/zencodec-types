//! Generic stub for unsupported codec operations.
//!
//! Use [`Unsupported<E>`] as the associated type for decode modes your codec
//! doesn't support, instead of defining custom stub types.

use core::marker::PhantomData;

use crate::{DecodeFrame, ImageInfo, OutputInfo, UnsupportedOperation};
use zenpixels::PixelSlice;

use super::decoder::{FrameDecode, StreamingDecode};

/// Stub type for codecs that don't support an operation.
///
/// Use as the associated type for unsupported decode modes:
///
/// ```rust,ignore
/// impl<'a> DecodeJob<'a> for MyDecodeJob<'a> {
///     type Error = MyError;
///     type Dec = MyDecoder<'a>;
///     type StreamDec = Unsupported<MyError>;
///     type FrameDec = Unsupported<MyError>;
///     // ...
///
///     fn streaming_decoder(self, ..) -> Result<Unsupported<MyError>, MyError> {
///         Err(UnsupportedOperation::RowLevelDecode.into())
///     }
///
///     fn frame_decoder(self, ..) -> Result<Unsupported<MyError>, MyError> {
///         Err(UnsupportedOperation::AnimationDecode.into())
///     }
/// }
/// ```
///
/// The job's method returns `Err(...)` before an `Unsupported` instance is
/// ever created, so the trait methods below are unreachable in practice.
///
/// Requires `E: From<UnsupportedOperation>` so that if the methods are
/// somehow called, they return proper errors.
pub struct Unsupported<E>(PhantomData<fn() -> E>);

impl<E: core::error::Error + Send + Sync + 'static + From<UnsupportedOperation>> StreamingDecode
    for Unsupported<E>
{
    type Error = E;

    fn next_batch(&mut self) -> Result<Option<(u32, PixelSlice<'_>)>, E> {
        Err(UnsupportedOperation::RowLevelDecode.into())
    }

    fn info(&self) -> &ImageInfo {
        unreachable!("Unsupported: streaming decode stub should never be constructed")
    }
}

impl<E: core::error::Error + Send + Sync + 'static + From<UnsupportedOperation>> FrameDecode
    for Unsupported<E>
{
    type Error = E;

    fn next_frame(&mut self) -> Result<Option<DecodeFrame>, E> {
        Err(UnsupportedOperation::AnimationDecode.into())
    }

    fn next_frame_to_sink(
        &mut self,
        _sink: &mut dyn crate::DecodeRowSink,
    ) -> Result<Option<OutputInfo>, E> {
        Err(UnsupportedOperation::AnimationDecode.into())
    }
}
