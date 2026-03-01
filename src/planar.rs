//! Planar pixel format descriptors and plane masks.
//!
//! Provides types for describing planar pixel layouts (separate planes for
//! luma, chroma, alpha, etc.) without coupling to buffer allocation.
//!
//! Buffer types (`PlaneSlice`, `PlanarSlice`, `PlanarBuffer`) are pipeline
//! concerns and live in zenimage, not here.

use crate::buffer::{ChannelType, TransferFunction};
use crate::pixel_format::{Subsampling, YuvMatrix};

/// Maximum number of planes in a planar layout.
pub const MAX_PLANES: usize = 8;

// ---------------------------------------------------------------------------
// PlaneSemantic
// ---------------------------------------------------------------------------

/// Semantic meaning of a plane in a planar layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum PlaneSemantic {
    /// Luminance (Y in YCbCr).
    Luma = 0,
    /// Blue-difference chroma (Cb / U).
    ChromaCb = 1,
    /// Red-difference chroma (Cr / V).
    ChromaCr = 2,
    /// Red channel.
    Red = 3,
    /// Green channel.
    Green = 4,
    /// Blue channel.
    Blue = 5,
    /// Alpha / transparency.
    Alpha = 6,
    /// Oklab lightness.
    OklabL = 7,
    /// Oklab green-red axis.
    OklabA = 8,
    /// Oklab blue-yellow axis.
    OklabB = 9,
    /// UltraHDR gain map.
    GainMap = 10,
    /// Single-channel grayscale.
    Gray = 11,
}

// ---------------------------------------------------------------------------
// PlaneSpec
// ---------------------------------------------------------------------------

/// Descriptor for a single plane in a planar layout.
///
/// Each plane has a semantic meaning, subsampling factors relative to
/// the reference (luma) dimensions, and its own channel type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaneSpec {
    /// What this plane represents.
    pub semantic: PlaneSemantic,
    /// Horizontal subsampling divisor (1 = full, 2 = half, 4 = quarter).
    pub h_subsample: u8,
    /// Vertical subsampling divisor (1 = full, 2 = half).
    pub v_subsample: u8,
    /// Channel storage type for this plane (usually same for all planes).
    pub channel_type: ChannelType,
}

impl PlaneSpec {
    /// Create a full-resolution plane spec.
    pub const fn full(semantic: PlaneSemantic, channel_type: ChannelType) -> Self {
        Self {
            semantic,
            h_subsample: 1,
            v_subsample: 1,
            channel_type,
        }
    }

    /// Create a subsampled plane spec.
    pub const fn subsampled(
        semantic: PlaneSemantic,
        h_sub: u8,
        v_sub: u8,
        channel_type: ChannelType,
    ) -> Self {
        Self {
            semantic,
            h_subsample: h_sub,
            v_subsample: v_sub,
            channel_type,
        }
    }

    /// Compute the width of this plane given the reference (luma) width.
    ///
    /// Rounds up using `div_ceil`.
    #[inline]
    pub const fn plane_width(self, ref_w: u32) -> u32 {
        ref_w.div_ceil(self.h_subsample as u32)
    }

    /// Compute the height of this plane given the reference (luma) height.
    ///
    /// Rounds up using `div_ceil`.
    #[inline]
    pub const fn plane_height(self, ref_h: u32) -> u32 {
        ref_h.div_ceil(self.v_subsample as u32)
    }

    /// Whether this plane is subsampled in either dimension.
    #[inline]
    pub const fn is_subsampled(self) -> bool {
        self.h_subsample > 1 || self.v_subsample > 1
    }
}

// ---------------------------------------------------------------------------
// PlanarDescriptor
// ---------------------------------------------------------------------------

/// Fixed-size planar format descriptor. Up to [`MAX_PLANES`] planes, no heap.
///
/// Describes the semantic layout, subsampling, and channel types of a
/// planar pixel format. Used for format negotiation and buffer allocation
/// planning — does not carry pixel data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlanarDescriptor {
    planes: [PlaneSpec; MAX_PLANES],
    plane_count: u8,
    /// YCbCr matrix coefficients (if applicable).
    pub yuv_matrix: YuvMatrix,
    /// Transfer function.
    pub transfer: TransferFunction,
}

impl PlanarDescriptor {
    /// Create from a slice of plane specs. Panics if `specs.len() > MAX_PLANES`.
    pub const fn new(
        specs: &[PlaneSpec],
        yuv_matrix: YuvMatrix,
        transfer: TransferFunction,
    ) -> Self {
        assert!(specs.len() <= MAX_PLANES, "too many planes");
        let mut planes = [PlaneSpec::full(PlaneSemantic::Gray, ChannelType::U8); MAX_PLANES];
        let mut i = 0;
        while i < specs.len() {
            planes[i] = specs[i];
            i += 1;
        }
        Self {
            planes,
            plane_count: specs.len() as u8,
            yuv_matrix,
            transfer,
        }
    }

    /// Number of planes.
    #[inline]
    pub const fn plane_count(&self) -> u8 {
        self.plane_count
    }

    /// Slice of active plane specs.
    #[inline]
    pub fn planes(&self) -> &[PlaneSpec] {
        &self.planes[..self.plane_count as usize]
    }

    /// Get a specific plane spec by index.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= plane_count`.
    #[inline]
    pub const fn plane(&self, idx: usize) -> PlaneSpec {
        assert!(idx < self.plane_count as usize, "plane index out of bounds");
        self.planes[idx]
    }

    /// Compute the width of a specific plane given reference width.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= plane_count`.
    #[inline]
    pub const fn plane_width(&self, idx: usize, ref_w: u32) -> u32 {
        self.plane(idx).plane_width(ref_w)
    }

    /// Compute the height of a specific plane given reference height.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= plane_count`.
    #[inline]
    pub const fn plane_height(&self, idx: usize, ref_h: u32) -> u32 {
        self.plane(idx).plane_height(ref_h)
    }

    // --- Factory methods for common formats ----------------------------------

    /// YCbCr 4:2:0 (3 planes: Y full, Cb half, Cr half).
    pub const fn ycbcr_420(channel_type: ChannelType, yuv_matrix: YuvMatrix) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Luma, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCb, 2, 2, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCr, 2, 2, channel_type),
            ],
            yuv_matrix,
            TransferFunction::Srgb,
        )
    }

    /// YCbCr 4:2:2 (3 planes: Y full, Cb h-half, Cr h-half).
    pub const fn ycbcr_422(channel_type: ChannelType, yuv_matrix: YuvMatrix) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Luma, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCb, 2, 1, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCr, 2, 1, channel_type),
            ],
            yuv_matrix,
            TransferFunction::Srgb,
        )
    }

    /// YCbCr 4:4:4 (3 planes, all full resolution).
    pub const fn ycbcr_444(channel_type: ChannelType, yuv_matrix: YuvMatrix) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Luma, channel_type),
                PlaneSpec::full(PlaneSemantic::ChromaCb, channel_type),
                PlaneSpec::full(PlaneSemantic::ChromaCr, channel_type),
            ],
            yuv_matrix,
            TransferFunction::Srgb,
        )
    }

    /// YCbCr 4:1:1 (3 planes: Y full, Cb quarter-h, Cr quarter-h).
    pub const fn ycbcr_411(channel_type: ChannelType, yuv_matrix: YuvMatrix) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Luma, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCb, 4, 1, channel_type),
                PlaneSpec::subsampled(PlaneSemantic::ChromaCr, 4, 1, channel_type),
            ],
            yuv_matrix,
            TransferFunction::Srgb,
        )
    }

    /// Planar RGB (3 planes, all full resolution).
    pub const fn planar_rgb(channel_type: ChannelType) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Red, channel_type),
                PlaneSpec::full(PlaneSemantic::Green, channel_type),
                PlaneSpec::full(PlaneSemantic::Blue, channel_type),
            ],
            YuvMatrix::Identity,
            TransferFunction::Srgb,
        )
    }

    /// Planar RGBA (4 planes, all full resolution).
    pub const fn planar_rgba(channel_type: ChannelType) -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::Red, channel_type),
                PlaneSpec::full(PlaneSemantic::Green, channel_type),
                PlaneSpec::full(PlaneSemantic::Blue, channel_type),
                PlaneSpec::full(PlaneSemantic::Alpha, channel_type),
            ],
            YuvMatrix::Identity,
            TransferFunction::Srgb,
        )
    }

    /// Planar Oklab (3 planes: L, a, b — all full resolution f32).
    pub const fn oklab() -> Self {
        Self::new(
            &[
                PlaneSpec::full(PlaneSemantic::OklabL, ChannelType::F32),
                PlaneSpec::full(PlaneSemantic::OklabA, ChannelType::F32),
                PlaneSpec::full(PlaneSemantic::OklabB, ChannelType::F32),
            ],
            YuvMatrix::Identity,
            TransferFunction::Linear,
        )
    }

    /// Create from a [`Subsampling`] enum value and matrix.
    pub const fn from_subsampling(
        sub: Subsampling,
        channel_type: ChannelType,
        yuv_matrix: YuvMatrix,
    ) -> Self {
        match sub {
            Subsampling::S444 => Self::ycbcr_444(channel_type, yuv_matrix),
            Subsampling::S422 => Self::ycbcr_422(channel_type, yuv_matrix),
            Subsampling::S420 => Self::ycbcr_420(channel_type, yuv_matrix),
            Subsampling::S411 => Self::ycbcr_411(channel_type, yuv_matrix),
        }
    }
}

// ---------------------------------------------------------------------------
// PlaneMask
// ---------------------------------------------------------------------------

/// Bitfield selecting which planes to operate on.
///
/// Used by operations that can selectively process planes (e.g., sharpen
/// luma only, pass through chroma).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaneMask {
    bits: u8,
}

impl PlaneMask {
    /// All planes selected.
    pub const ALL: Self = Self { bits: 0xFF };

    /// No planes selected.
    pub const NONE: Self = Self { bits: 0 };

    /// Luma plane only (plane 0).
    pub const LUMA: Self = Self { bits: 1 };

    /// Chroma planes only (planes 1 + 2).
    pub const CHROMA: Self = Self { bits: 0b110 };

    /// Alpha plane only (plane 3 in YCbCrA or RGBA).
    pub const ALPHA: Self = Self { bits: 0b1000 };

    /// Select a single plane by index.
    #[inline]
    pub const fn single(idx: u8) -> Self {
        debug_assert!(idx < MAX_PLANES as u8, "plane index out of bounds");
        Self { bits: 1 << idx }
    }

    /// Whether this mask includes the given plane index.
    #[inline]
    pub const fn includes(self, idx: u8) -> bool {
        (self.bits & (1 << idx)) != 0
    }

    /// Union of two masks.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Intersection of two masks.
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Number of selected planes.
    #[inline]
    pub const fn count(self) -> u8 {
        self.bits.count_ones() as u8
    }

    /// Raw bits value.
    #[inline]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    /// Create from raw bits.
    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }
}

impl Default for PlaneMask {
    fn default() -> Self {
        Self::ALL
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_spec_dimensions() {
        let full = PlaneSpec::full(PlaneSemantic::Luma, ChannelType::U8);
        assert_eq!(full.plane_width(1920), 1920);
        assert_eq!(full.plane_height(1080), 1080);
        assert!(!full.is_subsampled());

        let half = PlaneSpec::subsampled(PlaneSemantic::ChromaCb, 2, 2, ChannelType::U8);
        assert_eq!(half.plane_width(1920), 960);
        assert_eq!(half.plane_height(1080), 540);
        assert!(half.is_subsampled());

        // Odd dimensions round up
        assert_eq!(half.plane_width(1921), 961);
        assert_eq!(half.plane_height(1081), 541);
    }

    #[test]
    fn plane_spec_quarter() {
        let quarter = PlaneSpec::subsampled(PlaneSemantic::ChromaCb, 4, 1, ChannelType::U8);
        assert_eq!(quarter.plane_width(1920), 480);
        assert_eq!(quarter.plane_width(1921), 481);
        assert_eq!(quarter.plane_height(1080), 1080);
        assert!(quarter.is_subsampled());
    }

    #[test]
    fn planar_descriptor_ycbcr_420() {
        let desc = PlanarDescriptor::ycbcr_420(ChannelType::U8, YuvMatrix::Bt601);
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane(0).semantic, PlaneSemantic::Luma);
        assert_eq!(desc.plane(1).semantic, PlaneSemantic::ChromaCb);
        assert_eq!(desc.plane(2).semantic, PlaneSemantic::ChromaCr);

        assert_eq!(desc.plane_width(0, 1920), 1920);
        assert_eq!(desc.plane_width(1, 1920), 960);
        assert_eq!(desc.plane_height(0, 1080), 1080);
        assert_eq!(desc.plane_height(1, 1080), 540);
    }

    #[test]
    fn planar_descriptor_ycbcr_422() {
        let desc = PlanarDescriptor::ycbcr_422(ChannelType::U8, YuvMatrix::Bt709);
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane_width(1, 1920), 960);
        assert_eq!(desc.plane_height(1, 1080), 1080); // no vertical subsampling
    }

    #[test]
    fn planar_descriptor_planar_rgb() {
        let desc = PlanarDescriptor::planar_rgb(ChannelType::F32);
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane(0).semantic, PlaneSemantic::Red);
        assert_eq!(desc.plane(1).semantic, PlaneSemantic::Green);
        assert_eq!(desc.plane(2).semantic, PlaneSemantic::Blue);
        // All full resolution
        for i in 0..3 {
            assert_eq!(desc.plane_width(i, 100), 100);
            assert_eq!(desc.plane_height(i, 100), 100);
        }
    }

    #[test]
    fn planar_descriptor_oklab() {
        let desc = PlanarDescriptor::oklab();
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane(0).semantic, PlaneSemantic::OklabL);
        assert_eq!(desc.plane(0).channel_type, ChannelType::F32);
        assert_eq!(desc.transfer, TransferFunction::Linear);
    }

    #[test]
    fn planar_descriptor_from_subsampling() {
        let desc = PlanarDescriptor::from_subsampling(
            Subsampling::S420,
            ChannelType::U8,
            YuvMatrix::Bt601,
        );
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane_width(1, 1920), 960);
        assert_eq!(desc.plane_height(1, 1080), 540);

        let desc = PlanarDescriptor::from_subsampling(
            Subsampling::S411,
            ChannelType::U8,
            YuvMatrix::Bt601,
        );
        assert_eq!(desc.plane_width(1, 1920), 480);
    }

    #[test]
    fn planar_descriptor_411() {
        let desc = PlanarDescriptor::ycbcr_411(ChannelType::U8, YuvMatrix::Bt601);
        assert_eq!(desc.plane_count(), 3);
        assert_eq!(desc.plane_width(1, 1920), 480);
        assert_eq!(desc.plane_height(1, 1080), 1080);
    }

    #[test]
    fn plane_mask_operations() {
        assert_eq!(PlaneMask::ALL.count(), 8);
        assert_eq!(PlaneMask::NONE.count(), 0);
        assert_eq!(PlaneMask::LUMA.count(), 1);
        assert_eq!(PlaneMask::CHROMA.count(), 2);
        assert_eq!(PlaneMask::ALPHA.count(), 1);

        assert!(PlaneMask::ALL.includes(0));
        assert!(PlaneMask::ALL.includes(7));
        assert!(!PlaneMask::NONE.includes(0));

        assert!(PlaneMask::LUMA.includes(0));
        assert!(!PlaneMask::LUMA.includes(1));

        assert!(!PlaneMask::CHROMA.includes(0));
        assert!(PlaneMask::CHROMA.includes(1));
        assert!(PlaneMask::CHROMA.includes(2));
    }

    #[test]
    fn plane_mask_union_intersection() {
        let luma_chroma = PlaneMask::LUMA.union(PlaneMask::CHROMA);
        assert_eq!(luma_chroma.count(), 3);
        assert!(luma_chroma.includes(0));
        assert!(luma_chroma.includes(1));
        assert!(luma_chroma.includes(2));
        assert!(!luma_chroma.includes(3));

        let just_luma = luma_chroma.intersection(PlaneMask::LUMA);
        assert_eq!(just_luma.count(), 1);
        assert!(just_luma.includes(0));
    }

    #[test]
    fn plane_mask_single() {
        for i in 0..8 {
            let mask = PlaneMask::single(i);
            assert_eq!(mask.count(), 1);
            assert!(mask.includes(i));
        }
    }

    #[test]
    fn plane_mask_default_is_all() {
        assert_eq!(PlaneMask::default(), PlaneMask::ALL);
    }

    #[test]
    fn plane_mask_bits_roundtrip() {
        let mask = PlaneMask::from_bits(0b10101);
        assert_eq!(mask.bits(), 0b10101);
        assert_eq!(mask.count(), 3);
    }

    #[test]
    fn planar_descriptor_planes_slice() {
        let desc = PlanarDescriptor::ycbcr_420(ChannelType::U8, YuvMatrix::Bt601);
        let planes = desc.planes();
        assert_eq!(planes.len(), 3);
        assert_eq!(planes[0].semantic, PlaneSemantic::Luma);
    }

    #[test]
    fn planar_descriptor_size() {
        // PlanarDescriptor should be reasonably small since it's Copy
        let size = core::mem::size_of::<PlanarDescriptor>();
        assert!(
            size <= 48,
            "PlanarDescriptor is {size} bytes, expected <= 48"
        );
    }
}
