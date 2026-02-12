//! Shared traits and types for zen* image codecs.
//!
//! This crate defines the common API surface that all zen* codecs implement:
//!
//! - [`Encoding`] / [`EncodingJob`] — config and per-operation encode traits
//! - [`Decoding`] / [`DecodingJob`] — config and per-operation decode traits
//! - [`EncodeOutput`] / [`DecodeOutput`] — unified output types
//! - [`PixelData`] — typed pixel buffer enum over `imgref::ImgVec`
//! - [`ImageInfo`] / [`ImageMetadata`] / [`Orientation`] — image metadata
//! - [`ImageFormat`] — format detection from magic bytes
//! - [`ResourceLimits`] — resource limit configuration
//!
//! Individual codecs (zenjpeg, zenwebp, zengif, zenavif) implement these traits
//! on their own config types. Format-specific methods live on the concrete types,
//! not on the traits.
//!
//! `zencodecs` provides multi-format dispatch and convenience entry points.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod format;
mod info;
mod limits;
mod orientation;
mod output;
mod pixel;
mod traits;

pub use format::ImageFormat;
pub use info::{ImageInfo, ImageMetadata};
pub use limits::ResourceLimits;
pub use orientation::Orientation;
pub use output::{DecodeFrame, DecodeOutput, EncodeOutput};
pub use pixel::PixelData;
pub use traits::{Decoding, DecodingJob, Encoding, EncodingJob};

// Re-exports for codec implementors and users.
pub use enough::{Stop, Unstoppable};
pub use imgref::{Img, ImgRef, ImgRefMut, ImgVec};
pub use rgb::alt::BGRA as Bgra;
pub use rgb::{Gray, Rgb, Rgba};
