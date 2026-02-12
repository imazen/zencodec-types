# zencodec-types + zen* Codec TODOs

Audit completed 2025-02-12. Decisions recorded here.

## zencodec-types (this crate)

### Breaking: Trait Changes

#### Split `probe()` into `probe_header()` / `probe_full()`
- `probe_header()` — MUST be cheap, header-parse only, O(header) not O(pixels)
- `probe_full()` — MAY be expensive, full parse allowed (frame counting, etc.)
- Current `probe()` has no cost contract: zenjpeg/zenwebp/zenpng are cheap, zenavif/zengif do full decodes
- **Files:** `src/traits.rs` (Decoding trait, ~line 170)

#### Add `Send + Sync` bounds to `Encoding` and `Decoding`
- Currently `Sized + Clone` only
- All concrete types are Send + Sync today, but it's not enforced
- Adding later would be breaking
- **Files:** `src/traits.rs` lines 30, 157

#### Add `CodecCapabilities` struct + `.capabilities()` method
- Return `&'static CodecCapabilities` from `Encoding`/`Decoding` traits
- Struct with getter methods so it's extensible over time without breaking
- Fields to cover:
  - Metadata support: `supports_icc()`, `supports_exif()`, `supports_xmp()` (per encode/decode)
  - Cancellation: `supports_encode_cancel()`, `supports_decode_cancel()`
  - Limits: which `ResourceLimits` fields the codec actually enforces
  - Probe cost: whether `probe_header` is truly cheap
  - Pixel formats: which `PixelData` variants are natively supported
  - Gray: `supports_native_gray()` (avoids 3x waste on encode)
- Replaces silent no-ops with discoverable behavior
- **Files:** new `src/capabilities.rs`, `src/traits.rs`, `src/lib.rs`

#### `with_limits` takes owned `ResourceLimits` (not `&ResourceLimits`)
- Currently `fn with_limits(self, limits: &ResourceLimits) -> Self`
- `ResourceLimits` is 56 bytes, should be `Copy`
- Take by value for consistency with builder pattern
- **Files:** `src/traits.rs` (all four traits), `src/limits.rs` (add `Copy` derive)

### Non-Breaking: Type Changes

#### Add `GrayF32` variant to `PixelData`
- Have `RgbF32`, `RgbaF32`, `Gray8`, `Gray16` but no `GrayF32`
- JXL decodes to float grayscale
- `#[non_exhaustive]` so this is additive
- Add conversion methods to match existing variants
- **Files:** `src/pixel.rs`

#### Fix `DecodeFrame` inconsistency with `DecodeOutput`
- Missing: `into_gray8()`, `as_rgb8()`, `as_rgba8()`, `as_bgra8()`, `as_gray8()`
- `DecodeOutput` has all of these, `DecodeFrame` only has `into_rgb8/rgba8/bgra8`
- **Files:** `src/output.rs` (~line 200)

#### Add missing derives
- `ImageInfo`: add `Eq` (all field types support it)
- `ImageMetadata`: add `PartialEq`, `Eq`
- `ResourceLimits`: add `PartialEq`, `Eq`, `Copy`
- `EncodeOutput`: add `PartialEq`, `Eq`
- **Files:** `src/info.rs`, `src/limits.rs`, `src/output.rs`

### Conversion Correctness

#### Reconsider 16-bit to 8-bit conversion
- Currently uses `>> 8` (truncation), not rounding
- Correct linear mapping: `(v * 255 + 128) / 65535`
- Consider: should we assume sRGB? Require explicit color space?
- Maybe these convenience conversions should document they're sRGB-assuming
- Or require callers to do explicit conversion with color space awareness
- **Decision needed:** require linear sRGB awareness, make conversion explicit
- **Files:** `src/pixel.rs` (all `>> 8` sites: ~lines 113, 128-132, 148-152, 233-238, 258-260, 318)

### Tests

#### Fill test gaps
- BGRA conversions (`to_bgra8`, `into_bgra8`)
- 16-bit pixel conversions (`Rgb16`, `Rgba16`, `Gray16`)
- `to_bytes()` for multiple variants
- `to_gray8()` from RGB/RGBA/BGRA sources (luminance correctness)
- `ImageInfo::display_width()`/`display_height()` with rotated orientations
- `ImageFormat::Display` impl
- `from_extension` edge cases (jfif, jpe, JPEG, empty string)
- `DecodeFrame` gray8 and as_* methods (once added)
- **Files:** `src/pixel.rs`, `src/info.rs`, `src/format.rs`, `src/output.rs`

### Documentation

#### Update design doc to match reality
- `api-design.md` still shows `with_quality`/`with_effort`/`with_lossless` on traits
- `PixelData` in doc has 7 variants, implementation has 9
- `Orientation` not in design doc
- `decode_into_*` methods not in design doc
- `ResourceLimits` struct replaces individual limit methods
- **Files:** `/home/lilith/work/zencodecs/api-design.md`

### Consider

#### Pixel conversion utilities
- `bytes_to_rgb`, `img_rgb_to_bytes` etc. are copy-pasted across zenjpeg/zenwebp
- Consider adding to zencodec-types as utility functions
- Weigh against: crate lightness, keeping this traits-only
- Could be a separate `zencodec-convert` micro-crate
- May be better as `PixelData` variants or methods on the existing types

---

## Codec-Specific Issues

### zengif

#### Return error on dimension overflow (u32 to u16)
- `img.width() as u16` silently truncates dimensions > 65535
- Should return an error instead
- **Files:** `/home/lilith/work/zengif/src/zencodec.rs`

#### Remove dead `quality: Option<f32>` field
- `GifEncoding` stores `quality: Option<f32>` that is never read
- Value is cast to `u8` and written to `inner.quality` instead
- **Files:** `/home/lilith/work/zengif/src/zencodec.rs`

#### Fix expensive `probe()` — split into probe_header/probe_full
- Currently decodes every frame just to count them
- `probe_header()` should parse GIF header only (width, height, has_animation)
- `probe_full()` can count frames
- **Files:** `/home/lilith/work/zengif/src/zencodec.rs`

### zenavif

#### Fix silent ICC/XMP metadata loss
- `with_metadata` only stores EXIF, silently drops ICC and XMP
- Users doing roundtrip encode lose color profiles without warning
- Should either: store and embed, or report via capabilities that it's unsupported
- **Files:** `/home/lilith/work/zenavif/src/zencodec.rs`

#### Fix expensive `probe()` — split into probe_header/probe_full
- Currently does a full image decode just to get width/height/has_alpha
- Should parse AVIF container headers only
- **Files:** `/home/lilith/work/zenavif/src/zencodec.rs`

### zenpng

#### Document stop token no-op
- `with_stop` silently ignores the token for both encode and decode
- Should be surfaced via capabilities
- **Files:** `/home/lilith/work/zenpng/src/zencodec.rs`

### zenjxl

#### Document decode stop token no-op
- Encode supports stop, decode silently ignores it
- Should be surfaced via capabilities
- **Files:** `/home/lilith/work/zenjxl/src/zencodec.rs`

### All Codecs

#### Standardize MSRV to 1.93
- Currently ranges from 1.75 (zengif) to 1.93 (zenavif)
- Align all to 1.93, edition 2024
- **Files:** `Cargo.toml` in each codec repo

#### Gray8 encode waste
- WebP, GIF, AVIF expand Gray8 to RGB (3x memory) even when unnecessary
- JPEG does this too despite supporting native grayscale
- Capabilities struct should advertise `supports_native_gray()`
- Codecs that support gray natively should handle it in the trait impl
- **Files:** `zencodec.rs` in each codec repo

#### Inconsistent ResourceLimits enforcement
- Some merge config+job limits, some override, some ignore fields
- Document expected behavior via capabilities
- Log when limits are requested but not enforced
- **Files:** `zencodec.rs` in each codec repo

---

## Not Doing (Decided Against)

- **Quality/effort/lossless on trait** — leave on concrete types. Correct decision.
- **`Deref<Target=[u8]>` on EncodeOutput** — `AsRef<[u8]>` is sufficient.
