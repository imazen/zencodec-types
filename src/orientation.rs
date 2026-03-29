//! EXIF orientation support.
//!
//! [`Orientation`] is re-exported from [`zenpixels`] — the canonical
//! definition for the zen ecosystem. [`OrientationHint`] is defined here
//! as codec-layer policy for how decoders should handle orientation.

pub use zenpixels::Orientation;

/// How the decoder should handle orientation during decode.
///
/// Replaces a simple `with_orientation_hint(Orientation)` with richer
/// semantics: the caller can request orientation correction plus
/// additional transforms, which the decoder can coalesce into a
/// single operation (e.g., JPEG lossless DCT rotation).
///
/// Pass to [`DecodeJob::with_orientation()`](crate::decode::DecodeJob::with_orientation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OrientationHint {
    /// Don't touch orientation. Report intrinsic orientation in
    /// [`ImageInfo::orientation`](crate::ImageInfo).
    #[default]
    Preserve,

    /// Resolve EXIF/container orientation to [`Identity`](Orientation::Identity).
    ///
    /// The decoder coalesces this with the decode operation when possible
    /// (e.g., JPEG lossless DCT transform). The output `ImageInfo` will
    /// report `Orientation::Identity`.
    Correct,

    /// Resolve EXIF orientation, then apply an additional transform.
    ///
    /// The decoder coalesces the combined operation when possible.
    /// For example, if EXIF says Rotate90 and the hint says Rotate180,
    /// the decoder applies Rotate270 in a single step.
    CorrectAndTransform(Orientation),

    /// Ignore EXIF orientation. Apply exactly this transform.
    ///
    /// The EXIF orientation is not consulted. The given transform is
    /// applied literally.
    ExactTransform(Orientation),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_hint_default_is_preserve() {
        assert_eq!(OrientationHint::default(), OrientationHint::Preserve);
    }

    #[test]
    fn orientation_hint_variants() {
        let _ = OrientationHint::Preserve;
        let _ = OrientationHint::Correct;
        let _ = OrientationHint::CorrectAndTransform(Orientation::Rotate90);
        let _ = OrientationHint::ExactTransform(Orientation::Rotate180);
    }
}
