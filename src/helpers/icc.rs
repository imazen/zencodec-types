//! ICC profile identification and pixel descriptor derivation.
//!
//! Normalized-hash lookup against well-known ICC profiles from web corpus
//! analysis, Compact-ICC, skcms, ICC.org, colord, Ghostscript, HP, Facebook,
//! Google, Kodak, libvips, moxcms, and zenpixels-convert.
//!
//! Before hashing, metadata-only header fields are zeroed (CMM type, creation
//! date, platform, manufacturer, model, creator, profile ID). This collapses
//! functionally identical profiles that differ only in metadata (e.g., GIMP
//! re-embeds the same sRGB TRC with a fresh timestamp on every export).
//! Safe across ICC v2.0–v4.4 and v5/iccMAX — verified against the spec.
//!
//! Each entry is verified against its reference EOTF for all 65536 u16 values
//! using `scripts/mega_test.rs` and cross-validated against moxcms.
//!
//! The [`IccMatchTolerance`] enum lets callers choose how closely the ICC
//! profile's TRC must match the reference curve, from pixel-exact (±1 u16)
//! to intent-based (±56 u16 for lossy compact profiles).

use crate::decode::SourceColor;
use zenpixels::{Cicp, ColorPrimaries, PixelDescriptor, PixelFormat, TransferFunction};

// ── Pixel descriptor derivation ────────────────────────────────────────────

/// Derive a [`PixelDescriptor`] that accurately describes decoded pixel data.
///
/// Codecs should call this when building `DecodeOutput` or `OutputInfo` to
/// ensure the descriptor's transfer function and color primaries match the
/// actual pixel values — not a hardcoded sRGB assumption.
///
/// # Priority
///
/// 1. If `corrected_to` is `Some`, the pixels were color-managed to that
///    target during decode. The descriptor reflects the target.
/// 2. If `source_color` has CICP metadata, the descriptor uses the CICP
///    transfer function and primaries (pixels are in the source color space).
/// 3. If `source_color` has an ICC profile, the normalized hash is checked
///    against well-known profiles (sRGB, P3, BT.2020, BT.709, Adobe RGB,
///    ProPhoto) using [`identify_well_known_icc`] with the given `tolerance`.
///    Unrecognized profiles yield `Unknown` transfer/primaries.
/// 4. No color metadata at all: assumes sRGB (legacy format convention).
///
/// Use [`IccMatchTolerance::Intent`] (the most common choice) to honor
/// the encoder's declared color space even when the ICC TRC is a lossy
/// approximation. Use [`IccMatchTolerance::Exact`] when computing
/// perceptual metrics where ±1 u16 matters.
pub fn descriptor_for_decoded_pixels(
    format: PixelFormat,
    source_color: &SourceColor,
    corrected_to: Option<&Cicp>,
    tolerance: IccMatchTolerance,
) -> PixelDescriptor {
    if let Some(target) = corrected_to {
        return target.to_descriptor(format);
    }

    if let Some(cicp) = source_color.cicp {
        return cicp.to_descriptor(format);
    }

    if let Some(ref icc) = source_color.icc_profile {
        if let Some((primaries, transfer)) = identify_well_known_icc(icc, tolerance) {
            return Cicp::SRGB
                .to_descriptor(format)
                .with_transfer(transfer)
                .with_primaries(primaries);
        }
        // Unknown ICC profile — can't map without a full CMS parse.
        return Cicp::SRGB
            .to_descriptor(format)
            .with_transfer(TransferFunction::Unknown)
            .with_primaries(ColorPrimaries::Unknown);
    }

    // No color metadata — assume sRGB (web/browser default).
    Cicp::SRGB.to_descriptor(format)
}

// ── Well-known ICC profile identification ──────────────────────────────────

/// FNV-1a 64-bit hash of ICC profile bytes with metadata normalization.
///
/// Zeroes metadata-only header fields before hashing so that functionally
/// identical profiles (same colorants + TRC) produce the same hash even if
/// they differ in creation date, CMM, platform, creator, or profile ID.
///
/// Zeroed ranges (all non-colorimetric per ICC spec v2.0–v5/iccMAX):
/// - bytes  4– 7: preferred CMM type (advisory hint)
/// - bytes 24–35: creation date/time (metadata)
/// - bytes 40–43: primary platform (advisory hint)
/// - bytes 48–55: device manufacturer + device model (identification)
/// - bytes 80–83: profile creator (identification)
/// - bytes 84–99: profile ID / reserved (MD5 in v4, zero in v2)
const fn fnv1a_64_normalized(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;

    // Phase 1: first 100 bytes with metadata zeroing.
    // Zeroed ranges: 4..8, 24..36, 40..44, 48..56, 80..100.
    let header_len = if data.len() < 100 { data.len() } else { 100 };
    let mut i = 0;
    while i < header_len {
        let b = if (i >= 4 && i < 8)
            || (i >= 24 && i < 36)
            || (i >= 40 && i < 44)
            || (i >= 48 && i < 56)
            || (i >= 80)
        {
            0u8
        } else {
            data[i]
        };
        hash ^= b as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }

    // Phase 2: remaining bytes — no conditionals, straight hash.
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }
    hash
}

/// Map table color_primaries code to [`ColorPrimaries`] enum.
fn cp_from_table(code: u8) -> ColorPrimaries {
    match code {
        1 => ColorPrimaries::Bt709,
        9 => ColorPrimaries::Bt2020,
        12 => ColorPrimaries::DisplayP3,
        // AdobeRgb (200) and ProPhoto (201) require zenpixels 0.3+
        200 | 201 => ColorPrimaries::Unknown,
        _ => ColorPrimaries::Unknown,
    }
}

/// Map table transfer_characteristics code to [`TransferFunction`] enum.
fn tc_from_table(code: u8) -> TransferFunction {
    match code {
        1 => TransferFunction::Bt709,
        3 => TransferFunction::Pq,
        4 => TransferFunction::Hlg,
        8 => TransferFunction::Linear,
        13 => TransferFunction::Srgb,
        // Gamma22 (200) and Gamma18 (201) require zenpixels 0.3+
        200 | 201 => TransferFunction::Unknown,
        _ => TransferFunction::Unknown,
    }
}

/// Maximum u16 TRC error tolerance for ICC profile identification.
///
/// Controls how closely an ICC profile's TRC must match the reference EOTF
/// to be accepted as a known profile. Every entry in the hash table stores
/// the measured max u16 error (verified against the authoritative EOTF for
/// all 65536 input values).
///
/// Use [`Exact`](Self::Exact) when pixel-level accuracy matters (e.g.,
/// computing perceptual metrics). Use [`Intent`](Self::Intent) when you
/// want to honor the encoder's intent — a "Compact sRGB" profile with
/// ±56 u16 LUT error was clearly meant to be sRGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[must_use]
pub enum IccMatchTolerance {
    /// ±1 u16 max — parametric v4 profiles only.
    Exact = 1,
    /// ±3 u16 max — includes v2-magic parametric approximations.
    Precise = 3,
    /// ±13 u16 max — includes v2-micro LUT profiles and iPhone P3.
    Approximate = 13,
    /// ±56 u16 max — honors encoder intent (e.g., sRGB-v2-nano, Facebook sRGB).
    Intent = 56,
}

/// Well-known RGB ICC profile table: `(normalized_hash, primaries, transfer, max_u16_err)`.
///
/// Sorted by normalized hash for binary search. Hashes use [`fnv1a_64_normalized`]
/// which zeroes metadata-only header fields before hashing.
///
/// Every entry verified against its reference EOTF for all 65536 u16 values
/// using `scripts/mega_test.rs` and cross-validated with moxcms.
///
/// Sources: web corpus (corpus-builder spider of 55,539 images), Compact-ICC-Profiles,
/// skcms (Google), ICC.org, colord (freedesktop), Ghostscript/Artifex, HP/Lino,
/// Facebook, Google Android, Kodak, libvips/nip2, moxcms, zenpixels-convert.
// (normalized_hash, color_primaries, transfer_characteristics, max_u16_error)
#[rustfmt::skip]
const KNOWN_ICC_PROFILES: &[(u64, u8, u8, u8)] = {
    const S: (u8, u8) = (1, 13);    // sRGB: CP=BT.709, TC=sRGB
    const P: (u8, u8) = (12, 13);   // P3: CP=DisplayP3, TC=sRGB TRC
    const R: (u8, u8) = (9, 1);     // BT.2020: CP=BT.2020, TC=BT.709
    const B: (u8, u8) = (1, 1);     // BT.709: CP=BT.709, TC=BT.709
    const A: (u8, u8) = (200, 200);  // Adobe RGB: CP=AdobeRgb, TC=Gamma22
    const PH: (u8, u8) = (201, 201); // ProPhoto: CP=ProPhoto, TC=Gamma18
    const SG22: (u8, u8) = (1, 200); // sRGB primaries + gamma 2.2
    const SG18: (u8, u8) = (1, 201); // sRGB primaries + gamma 1.8
    const RPQ: (u8, u8) = (9, 3);   // BT.2020 + PQ
    const RHLG: (u8, u8) = (9, 4);  // BT.2020 + HLG
    const PPQ: (u8, u8) = (12, 3);  // Display P3 + PQ
    // Globally sorted by normalized hash.
    include!("icc_table_rgb.inc")
};

/// Well-known grayscale ICC profile table: `(normalized_hash, transfer, max_u16_err)`.
///
/// Same normalization and verification as the RGB table, but for `GRAY` color space profiles.
// (normalized_hash, transfer_characteristics, max_u16_error)
#[rustfmt::skip]
const KNOWN_GRAY_ICC_PROFILES: &[(u64, u8, u8)] =
    include!("icc_table_gray.inc")
;

/// Identify a well-known ICC profile by normalized hash lookup.
///
/// Computes a normalized FNV-1a 64-bit hash (metadata fields zeroed) and
/// checks against tables of known RGB and grayscale ICC profiles.
///
/// The `tolerance` parameter controls how closely the profile's TRC must
/// match the reference EOTF. Each entry stores its measured max u16 error
/// (verified for all 65536 input values). Only entries within the tolerance
/// are returned.
///
/// Returns `Some((ColorPrimaries, TransferFunction))` for recognized profiles,
/// `None` for unknown ones. Grayscale profiles return `ColorPrimaries::Bt709`
/// (grayscale has no gamut, but sRGB white point is assumed).
///
/// This is a fast-path check (~100ns). For the long tail of vendor profiles,
/// use structural analysis via a CMS backend (e.g., `ColorManagement::identify_profile`).
pub fn identify_well_known_icc(
    icc_bytes: &[u8],
    tolerance: IccMatchTolerance,
) -> Option<(ColorPrimaries, TransferFunction)> {
    let hash = fnv1a_64_normalized(icc_bytes);

    // Try RGB table first.
    if let Ok(idx) = KNOWN_ICC_PROFILES.binary_search_by_key(&hash, |&(h, _, _, _)| h) {
        let (_, cp, tc, err) = KNOWN_ICC_PROFILES[idx];
        if err <= tolerance as u8 {
            return Some((cp_from_table(cp), tc_from_table(tc)));
        }
    }

    // Try grayscale table.
    if let Ok(idx) = KNOWN_GRAY_ICC_PROFILES.binary_search_by_key(&hash, |&(h, _, _)| h) {
        let (_, tc, err) = KNOWN_GRAY_ICC_PROFILES[idx];
        if err <= tolerance as u8 {
            return Some((ColorPrimaries::Bt709, tc_from_table(tc)));
        }
    }

    None
}

/// Check if an ICC profile is a known sRGB profile by normalized hash lookup.
///
/// Convenience wrapper around [`identify_well_known_icc`] — returns `true`
/// if the profile is sRGB within [`Intent`](IccMatchTolerance::Intent) tolerance.
pub fn icc_profile_is_srgb(icc_bytes: &[u8]) -> bool {
    identify_well_known_icc(icc_bytes, IccMatchTolerance::Intent)
        .is_some_and(|(cp, tc)| cp == ColorPrimaries::Bt709 && tc == TransferFunction::Srgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use zenpixels::{AlphaMode, Cicp, ColorPrimaries, SignalRange, TransferFunction};

    // ── Priority 4: no metadata → sRGB assumption ──────────────────────

    #[test]
    fn no_metadata_assumes_srgb() {
        let sc = SourceColor::default();
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
    }

    #[test]
    fn no_metadata_gray_assumes_srgb() {
        let sc = SourceColor::default();
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Gray8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
        assert_eq!(desc.pixel_format(), PixelFormat::Gray8);
    }

    #[test]
    fn no_metadata_rgba_assumes_srgb() {
        let sc = SourceColor::default();
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgba8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
        assert_eq!(desc.pixel_format(), PixelFormat::Rgba8);
        assert_eq!(desc.alpha(), Some(AlphaMode::Straight));
    }

    #[test]
    fn no_metadata_f32_assumes_srgb() {
        let sc = SourceColor::default();
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::RgbF32,
            &sc,
            None,
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
        assert_eq!(desc.pixel_format(), PixelFormat::RgbF32);
    }

    // ── Priority 2: CICP metadata ──────────────────────────────────────

    #[test]
    fn cicp_srgb_sets_srgb_descriptor() {
        let sc = SourceColor::default().with_cicp(Cicp::SRGB);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
    }

    #[test]
    fn cicp_p3_sets_descriptor() {
        let sc = SourceColor::default().with_cicp(Cicp::DISPLAY_P3);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::DisplayP3);
    }

    #[test]
    fn cicp_pq_sets_descriptor() {
        let sc = SourceColor::default().with_cicp(Cicp::BT2100_PQ);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::RgbaF32,
            &sc,
            None,
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Pq);
        assert_eq!(desc.primaries, ColorPrimaries::Bt2020);
    }

    #[test]
    fn cicp_hlg_sets_descriptor() {
        let sc = SourceColor::default().with_cicp(Cicp::BT2100_HLG);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::RgbF32,
            &sc,
            None,
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Hlg);
        assert_eq!(desc.primaries, ColorPrimaries::Bt2020);
    }

    #[test]
    fn cicp_takes_precedence_over_icc() {
        // When both CICP and ICC are present, CICP wins (per AVIF/HEIF spec).
        let fake_icc: Arc<[u8]> = Arc::from(alloc::vec![0u8; 64].into_boxed_slice());
        let sc = SourceColor::default()
            .with_cicp(Cicp::DISPLAY_P3)
            .with_icc_profile(fake_icc);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::DisplayP3);
    }

    #[test]
    fn cicp_preserves_pixel_format() {
        let sc = SourceColor::default().with_cicp(Cicp::DISPLAY_P3);
        for fmt in [
            PixelFormat::Rgb8,
            PixelFormat::Rgba8,
            PixelFormat::Gray8,
            PixelFormat::RgbF32,
            PixelFormat::Bgra8,
        ] {
            let desc = descriptor_for_decoded_pixels(fmt, &sc, None, IccMatchTolerance::Intent);
            assert_eq!(desc.pixel_format(), fmt, "format mismatch for {fmt:?}");
        }
    }

    // ── Priority 3: ICC profile ────────────────────────────────────────

    #[test]
    fn unknown_icc_yields_unknown_descriptor() {
        let fake_icc: Arc<[u8]> = Arc::from(alloc::vec![0u8; 64].into_boxed_slice());
        let sc = SourceColor::default().with_icc_profile(fake_icc);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Unknown);
        assert_eq!(desc.primaries, ColorPrimaries::Unknown);
    }

    #[test]
    fn unknown_icc_preserves_format_and_alpha() {
        let fake_icc: Arc<[u8]> = Arc::from(alloc::vec![99u8; 128].into_boxed_slice());
        let sc = SourceColor::default().with_icc_profile(fake_icc);

        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgba8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.pixel_format(), PixelFormat::Rgba8);
        assert_eq!(desc.alpha(), Some(AlphaMode::Straight));
        assert_eq!(desc.signal_range, SignalRange::Full);

        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.pixel_format(), PixelFormat::Rgb8);
        assert!(desc.alpha().is_none());
    }

    #[test]
    fn unknown_icc_gray_preserves_format() {
        let fake_icc: Arc<[u8]> = Arc::from(alloc::vec![42u8; 96].into_boxed_slice());
        let sc = SourceColor::default().with_icc_profile(fake_icc);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Gray8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.pixel_format(), PixelFormat::Gray8);
        assert_eq!(desc.transfer, TransferFunction::Unknown);
        assert_eq!(desc.primaries, ColorPrimaries::Unknown);
    }

    #[test]
    fn empty_icc_yields_unknown() {
        // Edge case: zero-length ICC profile is definitely not a known sRGB profile.
        let empty_icc: Arc<[u8]> = Arc::from(alloc::vec![].into_boxed_slice());
        let sc = SourceColor::default().with_icc_profile(empty_icc);
        let desc =
            descriptor_for_decoded_pixels(PixelFormat::Rgb8, &sc, None, IccMatchTolerance::Intent);
        assert_eq!(desc.transfer, TransferFunction::Unknown);
        assert_eq!(desc.primaries, ColorPrimaries::Unknown);
    }

    // ── Priority 1: corrected_to overrides everything ──────────────────

    #[test]
    fn corrected_to_overrides_source_cicp() {
        let sc = SourceColor::default().with_cicp(Cicp::DISPLAY_P3);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::Rgb8,
            &sc,
            Some(&Cicp::SRGB),
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
    }

    #[test]
    fn corrected_to_overrides_unknown_icc() {
        let fake_icc: Arc<[u8]> = Arc::from(alloc::vec![0u8; 64].into_boxed_slice());
        let sc = SourceColor::default().with_icc_profile(fake_icc);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::Rgb8,
            &sc,
            Some(&Cicp::SRGB),
            IccMatchTolerance::Intent,
        );
        // corrected_to wins over unknown ICC → sRGB
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
    }

    #[test]
    fn corrected_to_overrides_no_metadata() {
        let sc = SourceColor::default();
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::Rgb8,
            &sc,
            Some(&Cicp::SRGB),
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::Bt709);
    }

    #[test]
    fn corrected_to_p3_target() {
        // Unusual but valid: color-corrected to P3 instead of sRGB.
        let sc = SourceColor::default().with_cicp(Cicp::SRGB);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::Rgb8,
            &sc,
            Some(&Cicp::DISPLAY_P3),
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.transfer, TransferFunction::Srgb);
        assert_eq!(desc.primaries, ColorPrimaries::DisplayP3);
    }

    #[test]
    fn corrected_to_preserves_format() {
        let sc = SourceColor::default().with_cicp(Cicp::BT2100_PQ);
        let desc = descriptor_for_decoded_pixels(
            PixelFormat::Bgra8,
            &sc,
            Some(&Cicp::SRGB),
            IccMatchTolerance::Intent,
        );
        assert_eq!(desc.pixel_format(), PixelFormat::Bgra8);
        assert_eq!(desc.transfer, TransferFunction::Srgb);
    }

    // ── identify_well_known_icc ───────────────────────────────────────

    #[test]
    fn identify_rejects_empty() {
        assert!(identify_well_known_icc(&[], IccMatchTolerance::Intent).is_none());
        assert!(!icc_profile_is_srgb(&[]));
    }

    #[test]
    fn identify_rejects_garbage() {
        assert!(identify_well_known_icc(&[0u8; 100], IccMatchTolerance::Intent).is_none());
    }

    #[test]
    fn identify_rejects_short() {
        assert!(identify_well_known_icc(&[1, 2, 3, 4], IccMatchTolerance::Intent).is_none());
    }

    #[test]
    fn icc_profile_is_srgb_compat() {
        assert!(!icc_profile_is_srgb(&[0u8; 100]));
    }

    #[test]
    fn fnv1a_deterministic() {
        let data = b"sRGB IEC61966-2.1";
        assert_eq!(fnv1a_64_normalized(data), fnv1a_64_normalized(data));
    }

    #[test]
    fn fnv1a_distinct_inputs() {
        assert_ne!(fnv1a_64_normalized(b"abc"), fnv1a_64_normalized(b"abd"));
    }

    #[test]
    fn known_rgb_profiles_table_sorted() {
        for i in 1..KNOWN_ICC_PROFILES.len() {
            assert!(
                KNOWN_ICC_PROFILES[i - 1].0 < KNOWN_ICC_PROFILES[i].0,
                "KNOWN_ICC_PROFILES not sorted at index {i}: 0x{:016x} >= 0x{:016x}",
                KNOWN_ICC_PROFILES[i - 1].0,
                KNOWN_ICC_PROFILES[i].0,
            );
        }
    }

    #[test]
    fn known_gray_profiles_table_sorted() {
        for i in 1..KNOWN_GRAY_ICC_PROFILES.len() {
            assert!(
                KNOWN_GRAY_ICC_PROFILES[i - 1].0 < KNOWN_GRAY_ICC_PROFILES[i].0,
                "KNOWN_GRAY_ICC_PROFILES not sorted at index {i}: 0x{:016x} >= 0x{:016x}",
                KNOWN_GRAY_ICC_PROFILES[i - 1].0,
                KNOWN_GRAY_ICC_PROFILES[i].0,
            );
        }
    }

    #[test]
    fn tolerance_levels_filter_correctly() {
        // Verify the table contains entries at various tolerance levels.
        let has_exact = KNOWN_ICC_PROFILES.iter().any(|e| e.3 <= 1);
        let has_intent = KNOWN_ICC_PROFILES.iter().any(|e| e.3 > 13 && e.3 <= 56);
        assert!(has_exact, "should have ±1 entries");
        assert!(has_intent, "should have intent-level (±14-56) entries");

        // Exact rejects anything >1, Intent accepts up to 56.
        for &(_, _, _, err) in KNOWN_ICC_PROFILES {
            if err > IccMatchTolerance::Exact as u8 {
                // This entry should be rejected by Exact but accepted by Intent
                assert!(err <= IccMatchTolerance::Intent as u8 || err > 56);
            }
        }
    }

    // ── Per-format decode scenarios (table-driven) ──────────────────
    //
    // Each row simulates the SourceColor a codec would produce and
    // verifies the descriptor. Covers JPEG, PNG, WebP, AVIF, JXL,
    // HEIC, GIF, BMP/PNM/Farbfeld, TIFF.

    fn sc_none() -> SourceColor {
        SourceColor::default()
    }
    fn sc_cicp(c: Cicp) -> SourceColor {
        SourceColor::default().with_cicp(c)
    }
    fn sc_icc(fill: u8, len: usize) -> SourceColor {
        let icc: Arc<[u8]> = Arc::from(alloc::vec![fill; len].into_boxed_slice());
        SourceColor::default().with_icc_profile(icc)
    }
    fn sc_cicp_icc(c: Cicp, fill: u8, len: usize) -> SourceColor {
        let icc: Arc<[u8]> = Arc::from(alloc::vec![fill; len].into_boxed_slice());
        SourceColor::default().with_cicp(c).with_icc_profile(icc)
    }

    use ColorPrimaries as CP;
    use TransferFunction as TF;

    type FormatScenario = (&'static str, PixelFormat, SourceColor, Option<Cicp>, TF, CP);

    #[test]
    fn format_scenarios() {
        // (name, pixel_format, source_color, corrected_to, expected_tf, expected_cp)
        let cases: &[FormatScenario] = &[
            // JPEG
            (
                "jpeg_no_icc",
                PixelFormat::Rgb8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "jpeg_unknown_icc",
                PixelFormat::Rgb8,
                sc_icc(0xCA, 3144),
                None,
                TF::Unknown,
                CP::Unknown,
            ),
            (
                "jpeg_corrected",
                PixelFormat::Rgb8,
                sc_icc(0xCA, 3144),
                Some(Cicp::SRGB),
                TF::Srgb,
                CP::Bt709,
            ),
            // PNG
            (
                "png_cicp_p3",
                PixelFormat::Rgba8,
                sc_cicp(Cicp::DISPLAY_P3),
                None,
                TF::Srgb,
                CP::DisplayP3,
            ),
            (
                "png_cicp_over_icc",
                PixelFormat::Rgba8,
                sc_cicp_icc(Cicp::DISPLAY_P3, 0, 100),
                None,
                TF::Srgb,
                CP::DisplayP3,
            ),
            (
                "png_no_metadata",
                PixelFormat::Rgba8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "png_hdr_pq",
                PixelFormat::Rgba16,
                sc_cicp(Cicp::BT2100_PQ),
                None,
                TF::Pq,
                CP::Bt2020,
            ),
            // WebP
            (
                "webp_no_icc",
                PixelFormat::Rgba8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "webp_unknown_icc",
                PixelFormat::Rgba8,
                sc_icc(0xA3, 480),
                None,
                TF::Unknown,
                CP::Unknown,
            ),
            // AVIF
            (
                "avif_srgb",
                PixelFormat::Rgba8,
                sc_cicp(Cicp::SRGB),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "avif_hdr10",
                PixelFormat::RgbaF32,
                sc_cicp(Cicp::BT2100_PQ),
                None,
                TF::Pq,
                CP::Bt2020,
            ),
            (
                "avif_hlg",
                PixelFormat::RgbF32,
                sc_cicp(Cicp::BT2100_HLG),
                None,
                TF::Hlg,
                CP::Bt2020,
            ),
            (
                "avif_p3",
                PixelFormat::Rgb8,
                sc_cicp(Cicp::DISPLAY_P3),
                None,
                TF::Srgb,
                CP::DisplayP3,
            ),
            // JXL
            (
                "jxl_srgb",
                PixelFormat::Rgb8,
                sc_cicp(Cicp::SRGB),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "jxl_p3_pq",
                PixelFormat::RgbaF32,
                sc_cicp(Cicp::new(12, 16, 0, true)),
                None,
                TF::Pq,
                CP::DisplayP3,
            ),
            // HEIC
            (
                "heic_p3",
                PixelFormat::Rgba8,
                sc_cicp(Cicp::DISPLAY_P3),
                None,
                TF::Srgb,
                CP::DisplayP3,
            ),
            (
                "heic_hdr10",
                PixelFormat::RgbaF32,
                sc_cicp(Cicp::BT2100_PQ),
                None,
                TF::Pq,
                CP::Bt2020,
            ),
            // GIF / BMP / PNM / Farbfeld
            (
                "gif_srgb",
                PixelFormat::Rgba8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "bmp_srgb",
                PixelFormat::Rgb8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            (
                "pnm_gray",
                PixelFormat::Gray8,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
            // TIFF
            (
                "tiff_unknown_icc",
                PixelFormat::Rgb16,
                sc_icc(0x54, 7261),
                None,
                TF::Unknown,
                CP::Unknown,
            ),
            (
                "tiff_no_icc",
                PixelFormat::Rgb16,
                sc_none(),
                None,
                TF::Srgb,
                CP::Bt709,
            ),
        ];

        for &(name, fmt, ref sc, ref corrected, exp_tf, exp_cp) in cases {
            let desc = descriptor_for_decoded_pixels(
                fmt,
                sc,
                corrected.as_ref(),
                IccMatchTolerance::Intent,
            );
            assert_eq!(desc.transfer, exp_tf, "{name}: transfer");
            assert_eq!(desc.primaries, exp_cp, "{name}: primaries");
            assert_eq!(desc.pixel_format(), fmt, "{name}: format");
        }
    }

    // ── Hash table structure ───────────────────────────────────────────

    #[test]
    fn identify_family_counts() {
        let count = |cp: u8, tc: u8| {
            KNOWN_ICC_PROFILES
                .iter()
                .filter(|e| e.1 == cp && e.2 == tc)
                .count()
        };
        assert!(count(1, 13) >= 30, "sRGB: {}", count(1, 13));
        assert!(count(12, 13) >= 30, "Display P3: {}", count(12, 13));
        assert!(count(200, 200) >= 15, "Adobe RGB: {}", count(200, 200));
        assert!(
            KNOWN_ICC_PROFILES.len() >= 100,
            "total: {}",
            KNOWN_ICC_PROFILES.len()
        );
        assert!(
            KNOWN_GRAY_ICC_PROFILES.len() >= 10,
            "gray: {}",
            KNOWN_GRAY_ICC_PROFILES.len()
        );
    }

    // ── Hash table integrity ──────────────────────────────────────────

    #[test]
    fn no_duplicate_hashes() {
        let mut seen = alloc::collections::BTreeSet::new();
        for entry in KNOWN_ICC_PROFILES {
            assert!(seen.insert(entry.0), "duplicate hash 0x{:016x}", entry.0);
        }
    }

    #[test]
    fn all_errors_within_intent_tolerance() {
        for entry in KNOWN_ICC_PROFILES {
            assert!(
                entry.3 <= IccMatchTolerance::Intent as u8,
                "entry 0x{:016x} has err={} exceeding Intent({})",
                entry.0,
                entry.3,
                IccMatchTolerance::Intent as u8,
            );
        }
    }

    #[test]
    fn all_table_codes_map_to_known_variants() {
        for entry in KNOWN_ICC_PROFILES {
            let (_, cp, tc, _) = *entry;
            // Codes 200/201 (AdobeRGB, ProPhoto) require zenpixels 0.3+ variants
            if cp >= 200 || tc >= 200 {
                continue;
            }
            assert_ne!(
                cp_from_table(cp),
                ColorPrimaries::Unknown,
                "entry 0x{:016x} has unmapped CP={}",
                entry.0,
                cp
            );
            assert_ne!(
                tc_from_table(tc),
                TransferFunction::Unknown,
                "entry 0x{:016x} has unmapped TC={}",
                entry.0,
                tc
            );
        }
        for entry in KNOWN_GRAY_ICC_PROFILES {
            let (_, tc, _) = *entry;
            if tc >= 200 {
                continue;
            }
            assert_ne!(
                tc_from_table(tc),
                TransferFunction::Unknown,
                "gray entry 0x{:016x} has unmapped TC={}",
                entry.0,
                tc
            );
        }
    }

    #[test]
    fn zero_filled_data_no_false_positive() {
        // ICC profiles often have zero padding. Verify no zero-filled
        // buffer at any profile-typical size matches a table entry.
        for len in [
            410, 456, 480, 524, 548, 656, 736, 790, 2576, 3024, 3144, 6184, 20420,
        ] {
            let zeros = alloc::vec![0u8; len];
            assert!(
                identify_well_known_icc(&zeros, IccMatchTolerance::Intent).is_none(),
                "zeros({len}) falsely matched a profile"
            );
        }
    }

    #[test]
    fn tolerance_ordering() {
        assert!(IccMatchTolerance::Exact < IccMatchTolerance::Precise);
        assert!(IccMatchTolerance::Precise < IccMatchTolerance::Approximate);
        assert!(IccMatchTolerance::Approximate < IccMatchTolerance::Intent);
    }

    // ── Signal range ───────────────────────────────────────────────────

    #[test]
    fn all_paths_produce_full_range() {
        let cases: &[(SourceColor, Option<&Cicp>)] = &[
            (SourceColor::default(), None),
            (SourceColor::default().with_cicp(Cicp::SRGB), None),
            (SourceColor::default().with_cicp(Cicp::DISPLAY_P3), None),
            (SourceColor::default().with_cicp(Cicp::BT2100_PQ), None),
            (SourceColor::default(), Some(&Cicp::SRGB)),
        ];
        for (sc, corrected) in cases {
            let desc = descriptor_for_decoded_pixels(
                PixelFormat::Rgb8,
                sc,
                *corrected,
                IccMatchTolerance::Intent,
            );
            assert_eq!(
                desc.signal_range,
                SignalRange::Full,
                "non-full range for {sc:?} corrected={corrected:?}"
            );
        }
    }
}
