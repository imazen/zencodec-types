# zencodec-types

Shared traits and types for the zen\* image codec family.

This crate defines the common interface that all zen\* codecs implement. It contains no codec logic — just traits, types, and pixel format descriptors. `no_std` compatible (requires `alloc`), `forbid(unsafe_code)`.

## Crates in the zen\* family

| Crate | Format | Repo |
|-------|--------|------|
| `zenjpeg` | JPEG | [imazen/zenjpeg](https://github.com/imazen/zenjpeg) |
| `zenwebp` | WebP | [imazen/zenwebp](https://github.com/imazen/zenwebp) |
| `zenpng` | PNG | [imazen/zenpng](https://github.com/imazen/zenpng) |
| `zengif` | GIF | [imazen/zengif](https://github.com/imazen/zengif) |
| `zenavif` | AVIF | [imazen/zenavif](https://github.com/imazen/zenavif) |
| `zenjxl` | JPEG XL | [imazen/zenjxl](https://github.com/imazen/zenjxl) |
| `zencodecs` | Multi-format dispatch | [imazen/zencodecs](https://github.com/imazen/zencodecs) |

## What's in this crate

Everything below is always available (`no_std + alloc`). Types marked **(codec)** require the `codec` feature (enabled by default), which pulls in `rgb`, `imgref`, and `enough`.

### Pixel format descriptors

**`PixelDescriptor`** — compact 5-byte struct describing a pixel format: channel type, layout, alpha mode, transfer function, and color primaries. This is the universal format tag used by `PixelBuffer`, `PixelSlice`, format negotiation (`supported_descriptors()`), and cost estimation.

Design rationale: image processing pipelines need to reason about pixel format without caring what concrete type holds the pixels. Packing five properties into a small `Copy` struct means you can compare formats, switch on them, and pass them through FFI boundaries without generic type parameters or trait objects. Named constants (`RGB8_SRGB`, `RGBAF32_LINEAR`, `BGRA8`, etc.) cover common formats; `with_transfer()` and `with_primaries()` let you resolve unknowns as metadata becomes available.

Weakness: `PixelDescriptor` tracks gamut (primaries) and transfer function but does not encode the full CICP tuple — matrix coefficients and range are absent. For full color description you still need `Cicp`. The descriptor also can't represent planar formats (YUV), packed formats (RGB565), or palette-indexed data. If a codec produces YUV internally, it must convert before exposing data through the trait surface.

The five component enums:

| Enum | Variants | Notes |
|------|----------|-------|
| `ChannelType` | `U8`, `U16`, `F16`, `I16`, `F32` | `F16`/`I16` for GPU and fixed-point pipelines. `byte_size()` returns per-channel bytes. |
| `ChannelLayout` | `Gray`, `GrayAlpha`, `Rgb`, `Rgba`, `Bgra` | No `Bgr` without alpha — Windows surfaces always have a fourth byte. |
| `AlphaMode` | `None`, `Straight`, `Premultiplied` | `None` on `Bgra` layout = BGRX (padding byte). |
| `TransferFunction` | `Linear`, `Srgb`, `Bt709`, `Pq`, `Hlg`, `Unknown` | `Unknown` is the default for raw decoded data. Must be resolved from CICP/ICC before color-sensitive operations. `from_cicp()` maps CICP transfer characteristic codes. |
| `ColorPrimaries` | `Bt709`, `Bt2020`, `DisplayP3`, `Unknown` | Discriminant values match CICP codes. `contains()` models gamut hierarchy (BT.2020 ⊃ P3 ⊃ BT.709) for the cost model — converting to a narrower gamut is lossy. |

**`BufferError`** — validation errors from `PixelBuffer`/`PixelSlice` construction: alignment violations, insufficient data, stride problems, invalid dimensions, format mismatches. Implements `core::error::Error`.

### Pixel storage

**`PixelBuffer`** — owned, format-erased pixel buffer. Wraps a `Vec<u8>` with width, height, stride, and a `PixelDescriptor`. Allocate with `new()`, wrap a pool-managed vec with `from_vec()`, recover it with `into_vec()` for reuse. Supports row access, sub-row slicing, zero-copy crop views, and SIMD-friendly stride alignment.

Design rationale: the alternative is `PixelData`, a 13-variant enum where every consumer must match all arms. `PixelBuffer` erases the format into a byte buffer tagged with a descriptor, so generic pipeline stages can operate on raw rows without monomorphization. The tradeoff is type safety — you work with `&[u8]` rows instead of `&[Rgb<u8>]`.

Weakness: `PixelBuffer` cannot represent pixel types that aren't byte-aligned or that need custom Drop logic. It also can't represent planar layouts. The `from_vec()` path trusts the caller's descriptor — there's no runtime check that the data actually matches the claimed format.

**`PixelSlice<'a>`** / **`PixelSliceMut<'a>`** — borrowed immutable / mutable views into pixel data with the same row/crop API as `PixelBuffer`. Zero-copy `From<ImgRef<T>>` impls exist for all rgb-crate pixel types (Rgb, Rgba, BGRA, Gray, and their u16/f32 variants). Optionally carry `ColorContext` (ICC/CICP metadata) and `WorkingColorSpace` (pipeline state tracking) so color metadata travels with pixel data through processing stages.

Weakness: `GrayAlpha<T>` does not implement `rgb::ComponentBytes`, so there's no zero-copy `From<ImgRef<GrayAlpha<T>>>`. Those paths must copy through `PixelData::to_bytes()`. The `From<ImgRef>` conversions always produce `TransferFunction::Unknown` — the caller must call `with_transfer()` or attach a `ColorContext` when the transfer function is known.

**`PixelData`** **(codec)** — `#[non_exhaustive]` enum with 13 variants over `ImgVec<T>`: `Rgb8`, `Rgba8`, `Bgra8`, `Gray8`, `Rgb16`, `Rgba16`, `Gray16`, `GrayAlpha8`, `GrayAlpha16`, `GrayAlphaF32`, `RgbF32`, `RgbaF32`, `GrayF32`. This is the typed pixel buffer used by `DecodeOutput` and `DecodeFrame`.

Design rationale: codecs produce pixels in their native format. A JPEG decoder returns `Rgb8`; a high-bit-depth AVIF returns `Rgba16` or `RgbaF32`. `PixelData` preserves the exact type so callers can borrow the data with zero copies (`as_rgb8()` returns `Option<ImgRef<Rgb<u8>>>`). The `non_exhaustive` attribute means new variants can be added without a semver break.

Weakness: 13 variants is a lot of match arms for generic consumers. `PixelData` does not convert between formats — it is a pure data container. If you need a specific format, use `Decoder::decode_into()` (the codec handles transfer function conversion correctly) or convert to `PixelBuffer` and work with raw bytes. `PixelData::descriptor()` always returns `TransferFunction::Unknown` because `ImgVec` carries no color metadata; use `DecodeOutput::descriptor()` or resolve manually from `ImageInfo::transfer_function()`.

**`GrayAlpha<T>`** **(codec)** — two-component `#[repr(C)]` pixel type with fields `v` (gray) and `a` (alpha). Owned by this crate rather than using `rgb::alt::GrayAlpha` to avoid API instability in the `rgb` crate.

Weakness: does not implement `rgb::ComponentBytes`, which blocks zero-copy conversion to `PixelSlice`. This is a conscious tradeoff — implementing it would require `unsafe` code.

### Image metadata

**`ImageInfo`** — everything known about an image from probing or decoding. Fields: dimensions, format, alpha, animation, frame count, bit depth, channel count, CICP, HDR metadata (ContentLightLevel, MasteringDisplay), ICC profile (`Arc<[u8]>` for cheap sharing), EXIF, XMP, orientation, gain map presence/metadata, and non-fatal warnings. Builder pattern with `with_*` methods.

Design rationale: a single struct covers the superset of what any codec might report. Optional fields (`Option<T>`) handle the fact that not every format provides every piece of metadata — a JPEG has no CICP, a PNG has no MasteringDisplay. ICC is stored as `Arc<[u8]>` so it can be shared across pipeline stages and pixel slices without cloning megabytes of ICC data.

Weakness: `ImageInfo` is large and keeps growing. Every new metadata type adds a field. The `warnings: Vec<String>` field allocates even for the common case (no warnings). EXIF and XMP are raw `Vec<u8>` — this crate doesn't parse them, so callers need a separate EXIF/XMP parser. There's also no way to represent per-frame metadata differences (all frames share one `ImageInfo` via `Arc`).

**`OutputInfo`** — predicted output from a decode operation, returned by `DecodeJob::output_info()`. Fields: width, height, native pixel format, has_alpha, orientation_applied, crop_applied. This is what your buffer must match.

Design rationale: decode hints (crop, scale, orientation) mean the output dimensions can differ from the stored dimensions. `OutputInfo` lets callers allocate the right buffer before decoding starts. `crop_applied` may differ from the crop hint because codecs round to block boundaries (JPEG MCU, AV1 superblock).

**`MetadataView<'a>`** — borrowed view of ICC/EXIF/XMP/CICP/HDR/orientation for encode roundtrip. Borrows byte slices from `ImageInfo` or caller-provided data. `Metadata` is the owned counterpart for crossing async/thread boundaries.

Design rationale: encoding typically happens right after decoding, so borrowing from the source `ImageInfo` avoids copying metadata bytes. The owned `Metadata` exists for pipelines where metadata must outlive the decoder (caches, async tasks).

Weakness: `MetadataView` has no field for gain map metadata. On encode, gain maps must be handled out-of-band through codec-specific APIs. The `orientation` field is mutable-by-convention (callers set it to `Normal` after applying rotation), which is a bit of an API smell.

**`Cicp`** — CICP color description (ITU-T H.273): color primaries, transfer characteristics, matrix coefficients, full-range flag. Named constants for common combinations: `SRGB`, `DISPLAY_P3`, `BT2100_PQ`, `BT2100_HLG`. Human-readable name methods for each code.

Design rationale: CICP is the standard way modern image formats (AVIF, JXL, HEIF) and video codecs describe color. Four `u8` fields + one `bool` is compact and `Copy`. The raw code points are preserved so codecs can round-trip values the crate doesn't have named constants for.

Weakness: the matrix coefficients field is only meaningful for YCbCr content. For RGB images, it should be 0 (Identity), but some encoders write incorrect values. This crate preserves whatever the file says; it doesn't validate consistency.

**`ContentLightLevel`** / **`MasteringDisplay`** — HDR metadata types per CEA-861.3 and SMPTE ST 2086. `ContentLightLevel` carries MaxCLL and MaxFALL in nits. `MasteringDisplay` carries display primaries, white point, and luminance range in the spec's integer units (with `_f64()` and `_nits()` convenience methods).

Design rationale: these travel with the image through the pipeline and get embedded in the output file. Keeping them as separate structs (rather than fields on `Cicp`) matches how they're stored in containers — AVIF and HEIF have separate boxes for each.

**`Orientation`** — EXIF orientation values 1-8. `from_exif()` maps u16 tag values, `swaps_dimensions()` reports whether width/height are exchanged, `display_dimensions()` computes the effective display size.

Design rationale: orientation is one of those things every image pipeline must handle, and getting it wrong means rotated thumbnails. Having it as a first-class enum with dimension helpers prevents the common mistake of forgetting to swap width/height for 90/270 rotations.

### Color management

**`ColorProfileSource<'a>`** — unified reference to a source color profile: `Icc(&[u8])`, `Cicp(Cicp)`, or `Named(NamedProfile)`. Pass directly to a CMS backend.

**`NamedProfile`** — well-known color profiles: `Srgb`, `DisplayP3`, `Bt2020`, `Bt2020Pq`, `Bt2020Hlg`, `AdobeRgb`, `LinearSrgb`. `to_cicp()` converts to CICP codes when a standard mapping exists (returns `None` for Adobe RGB).

**`ColorContext`** — bundles ICC profile bytes (`Option<Arc<[u8]>>`) and CICP parameters into a single `Arc`-shareable context. Carried on `PixelSlice` so color metadata travels with pixel data through pipeline stages without per-strip cloning.

**`WorkingColorSpace`** — tracks what color space pixels are currently in: `Native` (as-decoded), `LinearSrgb`, `LinearRec2020`, or `Oklab`. Used by the pipeline planner to know what transforms have been applied.

Design rationale: separating color metadata from pixel data is intentional. Pixel buffers are format-erased bytes; color context is metadata that describes those bytes. `Arc` sharing means a single ICC profile allocation serves an entire pipeline.

Weakness: `ColorProfileSource` borrows ICC data, so it can't outlive the `ImageInfo` it came from. For long-lived references, use `ColorContext` (which owns via `Arc`). `NamedProfile` covers only 7 profiles; uncommon profiles (ProPhoto, ROMM, ACEScg) require the `Icc` variant. `WorkingColorSpace` only tracks four working spaces — pipelines using other spaces (ACES, ICtCp) would need to extend the enum or use `Native` as a catch-all.

### Format detection

**`ImageFormat`** — enum of supported formats: `Jpeg`, `WebP`, `Gif`, `Png`, `Avif`, `Jxl`, `Pnm`, `Bmp`, `Farbfeld`. `detect()` identifies format from magic bytes (needs 2-12 bytes depending on format). `from_extension()` maps file extensions case-insensitively. Also provides `mime_type()`, `extensions()`, `min_probe_bytes()`, and capability queries (`supports_lossy()`, `supports_lossless()`, `supports_animation()`, `supports_alpha()`).

Design rationale: format detection belongs in the shared types crate because every pipeline entry point needs it, and magic byte detection is codec-independent. The `non_exhaustive` attribute means new formats can be added without breaking callers.

Weakness: `detect()` only checks the first bytes — it doesn't validate the file is actually well-formed. AVIF detection only checks for `avif`/`avis` ftyp brands; HEIF images with different brands won't match. JPEG detection can false-positive on some binary files that happen to start with `FF D8 FF`.

### Resource management

**`ResourceLimits`** — caps on resource usage: max pixels, max memory, max output size, max width/height, max file size, max frames, max animation duration. All fields `Option` — `None` means no limit. `Copy` so it's cheap to pass around. Validation methods: `check_dimensions()`, `check_memory()`, `check_file_size()`, `check_output_size()`, `check_frames()`, `check_duration()`, plus composite checks `check_image_info()`, `check_output_info()`, `check_decode_cost()`, `check_encode_cost()`.

Design rationale: server-side image processing needs hard limits to prevent resource exhaustion from malicious or oversized inputs. Having limits as a first-class type (rather than ad-hoc checks) means every codec enforces them consistently, and callers can validate at multiple stages — fast rejection from `probe_header()`, then refined rejection from `estimated_cost()`.

Weakness: codecs enforce what they can, but not all codecs support all limit types. `CodecCapabilities` reports which limits are enforced, but a codec that claims `enforces_max_memory: false` silently ignores the limit rather than returning an error. The caller must do their own `check_*` validation before calling into the codec for guaranteed protection.

**`LimitExceeded`** — error enum with variant-specific `actual`/`max` fields: `Width`, `Height`, `Pixels`, `Memory`, `FileSize`, `OutputSize`, `Frames`, `Duration`. Implements `core::error::Error`.

**`DecodeCost`** / **`EncodeCost`** — estimated resource costs. `DecodeCost` has `output_bytes`, `pixel_count`, and optional `peak_memory`. `EncodeCost` has `input_bytes`, `pixel_count`, and optional `peak_memory`. When `peak_memory` is `None`, the `check_*` methods fall back to buffer size as a lower-bound estimate.

Weakness: `peak_memory` accuracy varies wildly by codec. Some codecs can predict it well (JPEG); others have highly variable memory usage depending on content and settings (JPEG XL lossy can range from 6x to 22x input size). The fallback to buffer size can significantly undercount actual memory usage.

### Capability discovery

**`CodecCapabilities`** — static feature flags returned by each codec via `capabilities()`. ~30 boolean flags covering metadata support, animation, bit depth, alpha, cancellation, HDR, decode paths, encode paths, and limit enforcement. Plus `effort_range()` and `quality_range()` for parameter bounds. Constructed via `const fn` builder pattern for static initialization.

Design rationale: callers need to know what a codec supports before calling methods that might be no-ops or return `UnsupportedOperation`. A `&'static` reference means zero runtime cost. The builder pattern with `with_*` methods enables clean static construction in codec crates.

Weakness: capabilities are self-reported — there's no compile-time enforcement that a codec setting `row_level_encode: true` actually implements `push_rows()`. A dishonest capability struct will cause runtime errors. The flag count keeps growing; it's approaching the point where a bitflag set might be cleaner, but const fn builder ergonomics would suffer.

**`UnsupportedOperation`** — enum identifying which operation failed: `RowLevelEncode`, `PullEncode`, `AnimationEncode`, `DecodeInto`, `RowLevelDecode`, etc. Implements `core::error::Error`.

**`HasUnsupportedOperation`** — opt-in trait for codec error types. Lets generic callers check `err.unsupported_operation()` without downcasting to the codec-specific error type.

### Gain map support

**`GainMapMetadata`** — ISO 21496-1 parameters describing how to combine a base image with a gain map image for adaptive HDR/SDR rendering. Per-channel `[f32; 3]` fields for gain range, gamma, and offsets; scalar fields for HDR capacity range. Used by JPEG UltraHDR, AVIF, and JXL.

Design rationale: gain maps are the industry's answer to "how do I ship one file that looks good on both SDR and HDR displays." The metadata format is standardized across three major formats, so it belongs in the shared types crate rather than being duplicated in each codec.

Weakness: the gain map pixel data is accessed through `DecodeOutput::extras` via `Box<dyn Any>` downcast — there are no trait methods for gain map encoding/decoding yet. Callers must know the concrete extras type to downcast. This will be promoted to proper trait methods after the pattern is proven across multiple codecs. There's also no streaming API for gain maps — you can't decode the base image and gain map in parallel strips.

### I/O types

**`EncodeOutput`** — encoded bytes (`Vec<u8>`) plus `ImageFormat`. `into_vec()` recovers the vec, `bytes()` borrows, `AsRef<[u8]>` for direct use.

**`DecodeOutput`** **(codec)** — decoded pixels (`PixelData`) plus `ImageInfo` plus optional type-erased extras (`Box<dyn Any + Send>`). The `extras` field is the escape hatch for codec-specific secondary data (gain maps, MPF thumbnails) that doesn't fit the standard interface. `descriptor()` resolves transfer function from CICP automatically.

Weakness: `DecodeOutput` allocates the full image in memory. There's no streaming variant — for large images, use `Decoder::decode_rows()` or `Decoder::decode_into()` to avoid the allocation.

**`DecodeFrame`** **(codec)** — single animation frame with pixel data, `Arc<ImageInfo>`, delay, index, and compositing metadata (blend mode, disposal method, frame rect, required prior frame). The `Arc<ImageInfo>` means container-level metadata is shared across frames without duplication.

**`EncodeFrame<'a>`** — single frame for encoding with `PixelSlice`, duration, and compositing parameters. **`TypedEncodeFrame<'a, Pixel>`** **(codec)** is the generic-typed variant for convenience methods like `encode_animation_rgb8`.

**`FrameBlend`** — `Source` (replace) or `Over` (alpha-blend). **`FrameDisposal`** — `None` (leave as-is), `RestoreBackground`, or `RestorePrevious`. These map directly to GIF/APNG/WebP animation semantics.

**`DecodeRowSink`** **(codec)** — trait for zero-copy streaming decode. The codec calls `demand(y, height, width, bpp)` and the sink returns `(&mut [u8], stride)`. The sink controls stride (tight-packed or SIMD-aligned), and the codec writes directly into the provided buffer. Object-safe.

### Codec traits (codec feature)

**`EncoderConfig`** / **`DecoderConfig`** — reusable config types (`Clone + Send + Sync`, no lifetimes). Universal parameters on the trait: `with_calibrated_quality()`, `with_effort()`, `with_lossless()`. Format-specific settings on the concrete type. Create jobs with `job()`.

**`EncodeJob<'a>`** / **`DecodeJob<'a>`** — per-operation setup borrowing temporary data (stop tokens, metadata, limits) via `'a`. Create executors with `encoder()` / `decoder()` / `frame_encoder()` / `frame_decoder()`.

**`Encoder`** / **`Decoder`** — single-image executors with three paths each (one-shot, row-level, pull/push).

**`FrameEncoder`** / **`FrameDecoder`** — animation executors with three per-frame paths each.

Design rationale: the four-level hierarchy (config → job → executor) separates concerns cleanly. Configs are reusable and thread-safe. Jobs borrow per-operation data. Executors are consumed (or mutated for animation). GATs on the config traits (`type Job<'a>`) enable this without boxing.

Weakness: the trait surface is large. Each config trait requires `capabilities()`, `supported_descriptors()`, `format()`, `probe_header()`, plus a dozen convenience methods with defaults. Codec authors must implement a lot of boilerplate even for simple formats. The three-path design (one-shot / row-level / pull) means every executor has methods that some codecs can't meaningfully implement — those return `UnsupportedOperation`, but the caller has to check `CodecCapabilities` to know which paths are real.

### Error tracking (re-exports from `whereat`)

**`At<E>`** — wraps any error with file:line location. **`AtTrace`** / **`AtTraceable`** — trace chain support. **`ErrorAtExt`** / **`ResultAtExt`** — extension traits for `.at()` on errors and results. Codec error types use `type Error = At<MyCodecError>` so every error carries its source location.

### Re-exports (codec feature)

`imgref` (`Img`, `ImgRef`, `ImgRefMut`, `ImgVec`), `rgb` (the full crate, plus `Rgb`, `Rgba`, `Gray`, `Bgra` type aliases), `enough` (`Stop`, `Unstoppable` for cooperative cancellation).

## Architecture: Config → Job → Encoder/Decoder

Codecs use a four-level pattern:

```text
                              ┌→ Encoder (one-shot, row-push, or pull-from-source)
EncoderConfig → EncodeJob<'a> ┤
                              └→ FrameEncoder (animation: push frames, row-by-row, or pull)

                              ┌→ Decoder (one-shot, decode-into, or row callback)
DecoderConfig → DecodeJob<'a> ┤
                              └→ FrameDecoder (animation: pull frames, or row callback)
```

**Config** types (`EncoderConfig`, `DecoderConfig`) are reusable, `Clone + Send + Sync`, and have no lifetimes. Universal encoding parameters — `with_effort()`, `with_calibrated_quality()`, `with_lossless()` — are on the trait with default no-op implementations. Getters (`effort()`, `calibrated_quality()`, `is_lossless()`) return `Option` so callers can detect support. Format-specific settings beyond those live on the concrete type. You can store configs in structs, share them across threads, and create multiple jobs from one config.

**Job** types (`EncodeJob`, `DecodeJob`) are per-operation. They borrow temporary data (stop tokens, metadata, resource limits) via a `'a` lifetime and produce an executor.

**Executor** types (`Encoder`/`FrameEncoder`, `Decoder`/`FrameDecoder`) run the actual encode/decode. Single-image executors are consumed by one-shot methods. Animation executors are mutable and produce/consume frames iteratively.

```rust
use zenjpeg::{JpegEncoderConfig, JpegDecoderConfig};
use zencodec_types::{EncoderConfig, DecoderConfig, ResourceLimits};

// Config: universal quality/effort on the trait, format-specific on the concrete type
let config = JpegEncoderConfig::new()
    .with_calibrated_quality(85.0)
    .with_effort(1000);

// One-shot convenience: encode directly from config
let output = config.encode_rgb8(img.as_ref())?;

// Full pipeline: config → job → encoder → encode
let output = config.job()
    .with_metadata(&metadata)
    .with_stop(&stop_token)
    .encoder()
    .encode(PixelSlice::from(img.as_ref()))?;
```

### Calibrated quality scale

`EncoderConfig::with_calibrated_quality()` uses a calibrated 0.0–100.0 scale. The baseline is libjpeg-turbo: quality 85 on any codec targets the same visual quality (butteraugli / SSIM2 score) as libjpeg-turbo quality 85. Each codec maintains a calibration table mapping universal quality to its internal parameters.

| Universal quality | Visual target |
|---|---|
| 100.0 | Visually lossless (butteraugli < 0.5) |
| 85.0 | High quality web (matches libjpeg-turbo q85) |
| 75.0 | Standard web (matches libjpeg-turbo q75) |
| 50.0 | Moderate compression |
| 0.0 | Maximum compression |

`EncoderConfig::with_effort()` takes an `i32`. Higher = slower / better compression. Each codec maps this to its internal effort/speed parameter. `CodecCapabilities::effort_range()` reports the meaningful `[min, max]` — values outside it are clamped.

`EncoderConfig::with_lossless()` enables lossless encoding when supported (`CodecCapabilities::lossless()`). When lossless is enabled, `with_calibrated_quality()` is ignored.

### Encode paths

```rust
// One-shot encode (typed convenience)
config.encode_rgb8(img)?;

// One-shot encode (format-erased)
config.job().encoder().encode(PixelSlice::from(img))?;

// Row-level push: caller sends rows
let mut enc = config.job().with_metadata(&meta).encoder();
enc.push_rows(rows_0_to_63)?;
enc.push_rows(rows_64_to_127)?;
let output = enc.finish()?;

// Pull-from-source: encoder requests rows via callback
let output = config.job().encoder().encode_from(&mut |row_idx, buf| {
    fill_rows(row_idx, buf)  // return number of rows written, 0 = done
})?;

// Animation: push complete frames
let mut enc = config.job().frame_encoder()?;
enc.push_frame(PixelSlice::from(frame1), 100)?;
enc.push_frame(PixelSlice::from(frame2), 100)?;
let output = enc.finish()?;

// Animation: build frames row-by-row
let mut enc = config.job().frame_encoder()?;
enc.begin_frame(100)?;
enc.push_rows(rows)?;
enc.end_frame()?;
let output = enc.finish()?;

// Animation: pull rows per frame from callback
let mut enc = config.job().frame_encoder()?;
enc.pull_frame(100, &mut |row_idx, buf| fill_rows(row_idx, buf))?;
let output = enc.finish()?;

// Animation: sub-canvas frames with compositing
let mut enc = config.job()
    .with_canvas_size(320, 240)
    .frame_encoder()?;
enc.push_encode_frame(EncodeFrame::new(PixelSlice::from(region), 100)
    .with_frame_rect([10, 20, 64, 48])
    .with_blend(FrameBlend::Over)
    .with_disposal(FrameDisposal::RestoreBackground))?;
let output = enc.finish()?;
```

### Decode paths

```rust
// One-shot decode (typed convenience)
let output = config.decode(data)?;

// Decode into caller buffer (use output_info to size the buffer)
let info = config.job().output_info(data)?;
let mut buf = PixelBuffer::new(info.width, info.height, info.native_format)?;
config.job().decoder().decode_into(data, buf.as_mut_slice())?;

// Decode with hints: crop + prescale + orientation
let job = config.job()
    .with_crop_hint(100, 100, 800, 600)
    .with_scale_hint(400, 300)
    .with_orientation_hint(Orientation::Rotate90);
let info = job.output_info(data)?;  // ← dimensions reflect applied hints
let mut buf = PixelBuffer::new(info.width, info.height, info.native_format)?;
job.decoder().decode_into(data, buf.as_mut_slice())?;

// Row-level callback: decoder pushes rows as they're available
config.job().decoder().decode_rows(data, &mut |row_idx, row| {
    process_row(row_idx, row);
})?;

// Animation: pull complete frames
let mut dec = config.job().frame_decoder(data)?;
while let Some(frame) = dec.next_frame()? {
    // Frame carries compositing info for correct rendering
    let _blend = frame.blend();           // FrameBlend::Source or Over
    let _disposal = frame.disposal();     // FrameDisposal::None, RestoreBackground, RestorePrevious
    let _depends_on = frame.required_frame(); // None = keyframe, Some(n) = depends on frame n
    let _region = frame.frame_rect();     // None = full canvas, Some([x, y, w, h])
    process(frame);
}

// Animation: pull frames into caller buffer with prior-frame hint
let mut dec = config.job().frame_decoder(data)?;
let mut prior = None;
while let Some(info) = dec.next_frame_into(buf.as_mut_slice(), prior)? {
    // prior_frame hint tells the decoder the buffer already contains
    // a composited frame, enabling incremental compositing
    prior = Some(info.frame_index().unwrap_or(0));
    process_buffer(&buf, &info);
}

// Animation: row-level callback per frame
let mut dec = config.job().frame_decoder(data)?;
while let Some(info) = dec.next_frame_rows(&mut |row_idx, row| {
    process_row(row_idx, row);
})? {
    on_frame_complete(&info);
}
```

## Opaque pixel buffers

`PixelData` is a 13-variant enum — every consumer must match all 13 arms to do anything generic. The `buffer` module provides an alternative: format-erased buffers tagged with a 4-byte `PixelDescriptor`.

**`PixelDescriptor`** packs `ChannelType` (U8/U16/F32), `ChannelLayout` (Gray/GrayAlpha/Rgb/Rgba/Bgra), `AlphaMode` (None/Straight/Premultiplied), and `TransferFunction` (Linear/Srgb/Bt709/Pq/Hlg/Unknown) into 4 bytes. Two sets of named constants: transfer-agnostic (`RGB8`, `RGBF32`, etc. with `Unknown` transfer) and explicitly-tagged (`RGB8_SRGB`, `RGBF32_LINEAR`, etc.). Use `with_transfer()` to resolve `Unknown` once CICP/ICC is consulted.

**`PixelBuffer`** is an owned `Vec<u8>` with dimensions, stride, and descriptor. Allocate with `new()`, wrap a pool vec with `from_vec()`, recover it with `into_vec()`. Supports row access, sub-row slicing, and zero-copy crop views.

**`PixelSlice<'a>`** / **`PixelSliceMut<'a>`** are borrowed views with the same row/crop API. Zero-copy `From<ImgRef<T>>` and `From<ImgRefMut<T>>` impls exist for all 10 rgb-crate pixel types.

Conversions between `PixelData` and `PixelBuffer` always copy (no `unsafe` needed):

```rust
use zencodec_types::{PixelBuffer, PixelData, PixelDescriptor};

// PixelData → PixelBuffer
let buf = PixelBuffer::from(pixel_data);

// PixelBuffer → PixelData
let data = PixelData::try_from(buf)?;

// ImgRef → PixelSlice (zero-copy)
let slice = PixelSlice::from(img.as_ref());

// ImgRefMut → PixelSliceMut (zero-copy)
let slice_mut = PixelSliceMut::from(img.as_mut());
```

### Color profile source

`ImageInfo` and `MetadataView` carry raw color data (CICP codes, ICC bytes, HDR metadata). The `color_profile_source()` method returns a unified `ColorProfileSource` that consumers can pass directly to a CMS backend (e.g., moxcms):

```rust
use zencodec_types::{ColorProfileSource, NamedProfile};

let output = config.decode(data)?;

// Get source color space (CICP takes precedence over ICC per AVIF/HEIF spec)
let source = output.info().color_profile_source()
    .unwrap_or(ColorProfileSource::Named(NamedProfile::Srgb));

// Set up a CMS transform to your working color space
let target = ColorProfileSource::Named(NamedProfile::LinearSrgb);
let transform = cms.create_transform(source, target, layout)?;
```

`NamedProfile` covers the common profiles (sRGB, Display P3, BT.2020, BT.2020+PQ, BT.2020+HLG, Adobe RGB, Linear sRGB). Use `NamedProfile::to_cicp()` to convert to CICP codes when a standard mapping exists.

### Natural info vs output info

`ImageInfo` describes the file as stored — original dimensions, original orientation, embedded metadata. You get it from `probe_header()` or `probe_full()`.

`OutputInfo` describes what the decoder will actually produce — post-crop, post-scale, post-orientation dimensions and pixel format. **This is what your destination buffer must match.**

```rust
// Natural info: what's in the file
let natural = config.probe_header(data)?;
println!("Stored: {}x{}, orientation {:?}", natural.width, natural.height, natural.orientation);

// Output info with no hints: what decode() will produce by default
let output = config.job().output_info(data)?;

// Output info with hints: request crop + prescale + orientation
let output = config.job()
    .with_crop_hint(100, 100, 800, 600)   // request a region
    .with_scale_hint(400, 300)             // request prescaling
    .with_orientation_hint(natural.orientation)  // apply EXIF orientation
    .output_info(data)?;                   // → what buffer to allocate

// Allocate to match what the decoder will write
let mut buf = PixelBuffer::new(output.width, output.height, output.native_format)?;

// output.orientation_applied tells you what the decoder handled
// output.crop_applied tells you the actual crop (may differ from hint due to block alignment)
```

Decode hints are optional suggestions — the decoder may ignore them, apply them partially (e.g., block-aligned crop on JPEG MCU boundaries), or apply them fully. Always check `OutputInfo` to learn what the decoder will actually do.

| `OutputInfo` field | Meaning |
|---|---|
| `width`, `height` | Dimensions the decoder will write |
| `native_format` | Pixel format the decoder would pick for `decode()` |
| `has_alpha` | Whether the output has an alpha channel |
| `orientation_applied` | Orientation the decoder will handle (Normal = caller must handle it) |
| `crop_applied` | Actual crop region in source coordinates (may differ from hint) |

### Cost estimation and resource limits

Both decode and encode jobs provide cost estimation. Use `ResourceLimits` with the `check_*` methods for a complete resource management pipeline.

**Decode cost:**

```rust
let limits = ResourceLimits::none()
    .with_max_pixels(100_000_000)
    .with_max_memory(512 * 1024 * 1024);

// 1. Parse-time rejection (fastest — no pixel work yet)
let info = config.probe_header(data)?;
limits.check_image_info(&info)?;

// 2. Cost-aware rejection (after hints, before decode)
let job = config.job().with_crop_hint(0, 0, 1000, 1000);
let cost = job.estimated_cost(data)?;
limits.check_decode_cost(&cost)?;
```

**Encode cost:**

```rust
let cost = encode_job.estimated_cost(width, height, PixelDescriptor::RGBA8_SRGB);
limits.check_encode_cost(&cost)?;
```

`DecodeCost` and `EncodeCost` both carry `output_bytes`/`input_bytes`, `pixel_count`, and an optional `peak_memory`. When `peak_memory` is `None`, the `check_*` methods fall back to the buffer size as a lower-bound estimate.

Typical working memory multipliers over buffer size:

| Codec | Decode | Encode |
|-------|--------|--------|
| JPEG | ~1-2x | ~2-3x |
| PNG | ~1-2x | ~2x |
| WebP lossy | ~2x | ~3-4x |
| AV1/AVIF | ~2-3x | ~4-8x |
| JPEG XL (u8) | ~1-2x | ~6-22x |

`LimitExceeded` is a proper `Error` type with variant-specific `actual`/`max` fields for useful error messages. Individual `check_*` methods are available for fine-grained validation: `check_dimensions()`, `check_memory()`, `check_file_size()`, `check_output_size()`, `check_frames()`, `check_duration()`.

### Transfer function conventions

- **u8 / u16**: typically gamma-encoded (sRGB). u16 uses the full 0-65535 range.
- **f32**: typically linear light, but depends on the codec and source content.

`PixelData` does not track its transfer function — `descriptor()` returns `TransferFunction::Unknown`. The actual transfer function lives in `ImageInfo::cicp` (or the ICC profile). Use `ImageInfo::transfer_function()` to derive it from CICP, then `PixelDescriptor::with_transfer()` to tag the descriptor:

```rust
let output = config.decode(data)?;

// PixelData::descriptor() returns Unknown transfer
let desc = output.pixels().descriptor();
assert!(desc.is_unknown_transfer());

// Resolve from CICP metadata
let resolved = desc.with_transfer(output.info().transfer_function());
// resolved.transfer is Srgb, Pq, Hlg, etc. (or Unknown if no CICP)

// Or use DecodeOutput::descriptor() which does this automatically
let desc = output.descriptor(); // transfer resolved from CICP
```

The `From<ImgRef>` and `From<ImgRefMut>` conversions to `PixelSlice`/`PixelSliceMut` also use `Unknown` transfer — `ImgRef` doesn't carry color metadata, so the transfer function must be set by the caller when known.

`PixelData` does **not** convert between pixel formats. If you need a specific format, use `Decoder::decode_into()` to request it from the codec (the codec applies correct transfer functions internally). For format-erased processing, convert to `PixelBuffer` with `PixelBuffer::from(pixel_data)` and work with raw byte rows.

## Implementor reference

This section documents the contract that codec implementations must satisfy. If you're writing a new zen\* codec, read this carefully.

### Trait hierarchy

Each side (encode/decode) has four traits:

| Trait | Role | Bounds |
|-------|------|--------|
| `EncoderConfig` / `DecoderConfig` | Reusable config, typed convenience methods | `Clone + Send + Sync` |
| `EncodeJob<'a>` / `DecodeJob<'a>` | Per-operation setup (stop, metadata, limits) | `Sized` |
| `Encoder` / `Decoder` | Single-image execution | `Sized` |
| `FrameEncoder` / `FrameDecoder` | Animation execution | `Sized` |

Config types create jobs. Jobs create executors. Executors run the codec.

### `supported_descriptors()` — format negotiation (required)

Both `EncoderConfig` and `DecoderConfig` require a `supported_descriptors()` method (no default). This returns the pixel formats the codec handles natively, without any internal conversion.

**This is a hard contract:** if a descriptor is in the list, `encode()`/`decode_into()` with a matching buffer **must work** — zero conversion overhead, the codec processes the data directly. The list must not be empty.

```rust
impl EncoderConfig for MyJpegEncoder {
    fn supported_descriptors() -> &'static [PixelDescriptor] {
        &[PixelDescriptor::RGB8_SRGB, PixelDescriptor::GRAY8_SRGB]
    }
    // ...
}

impl DecoderConfig for MyJpegDecoder {
    fn supported_descriptors() -> &'static [PixelDescriptor] {
        // decode_into() with any of these must produce correct output directly
        &[PixelDescriptor::RGB8_SRGB, PixelDescriptor::GRAY8_SRGB]
    }
    // ...
}
```

Callers use this to pick the best pixel format before encoding/decoding, avoiding unnecessary conversions. The codec may also accept other formats via internal conversion, but the supported descriptors are the fast path.

Use `PixelDescriptor::layout_compatible()` to compare formats while ignoring transfer function and alpha mode differences.

### `capabilities()` — declare what you support

Both config traits require a `fn capabilities() -> &'static CodecCapabilities` method. This returns a static reference describing what the codec supports.

```rust
use zencodec_types::CodecCapabilities;

static ENCODE_CAPS: CodecCapabilities = CodecCapabilities::new()
    .with_encode_icc(true)
    .with_encode_exif(true)
    .with_encode_cancel(true)
    .with_cheap_probe(true);

impl EncoderConfig for MyEncoder {
    fn capabilities() -> &'static CodecCapabilities { &ENCODE_CAPS }
    // ...
}
```

The capabilities must be honest:

- **`encode_cancel` / `decode_cancel`**: Set to `true` only if `with_stop()` actually checks the token and can bail out early. If your `with_stop()` is a no-op, set to `false`.
- **`encode_icc` / `encode_exif` / `encode_xmp`**: Set to `true` only if `with_metadata()` actually embeds that metadata type in the output. Metadata types that are silently dropped must be `false`.
- **`decode_icc` / `decode_exif` / `decode_xmp`**: Set to `true` only if the decoder extracts that metadata into `ImageInfo`.
- **`native_gray`**: Set to `true` if the codec can encode/decode grayscale without expanding to RGB.
- **`cheap_probe`**: Set to `true` if `probe_header()` parses only container headers (O(header), not O(pixels)). This should be `true` for most codecs.

### `probe_header()` vs `probe_full()`

`probe_header()` is the only required probe method. It **must be cheap** — parse container/file headers to extract dimensions, format, and basic metadata. O(header), not O(pixels). Do not decode image data here.

`probe_full()` defaults to calling `probe_header()`. Override it only when getting complete metadata (like frame counts in animated formats) requires a more expensive parse. Document the cost.

Example for an animated format:

```rust
fn probe_header(&self, data: &[u8]) -> Result<ImageInfo, Self::Error> {
    // Parse GIF header + logical screen descriptor only.
    // Returns dimensions, format; frame_count will be None.
    parse_gif_header(data)
}

fn probe_full(&self, data: &[u8]) -> Result<ImageInfo, Self::Error> {
    // Walk all frames to count them. O(file_size).
    let mut info = self.probe_header(data)?;
    info.frame_count = Some(count_gif_frames(data)?);
    Ok(info)
}
```

### `output_info()` — predict output dimensions

`DecodeJob::output_info()` is the required method for predicting decode output. It returns `OutputInfo` with the width, height, pixel format, and applied transforms that `decode()` / `decode_into()` will produce. Callers use this to allocate destination buffers.

After applying hints (crop, scale, orientation), `output_info()` must reflect those transforms. If the codec ignores a hint, the corresponding `OutputInfo` field should indicate that (e.g., `orientation_applied` remains `Normal` if orientation is not handled).

For a simple codec that ignores all hints:

```rust
fn output_info(&self, data: &[u8]) -> Result<OutputInfo, Self::Error> {
    let info = parse_header(data)?;
    Ok(OutputInfo::full_decode(info.width, info.height, PixelDescriptor::RGB8_SRGB)
        .with_alpha(info.has_alpha))
}
```

### `estimated_cost()` — resource management

Both `DecodeJob` and `EncodeJob` provide `estimated_cost()` with defaults that compute buffer sizes. Override to provide codec-specific `peak_memory` estimates.

**Decode side** (default derives from `output_info()`):

```rust
fn estimated_cost(&self, data: &[u8]) -> Result<DecodeCost, Self::Error> {
    let info = self.output_info(data)?;
    Ok(DecodeCost {
        output_bytes: info.buffer_size(),
        pixel_count: info.pixel_count(),
        // AV1: ~3x output for tile buffers + CDEF + reference frames
        peak_memory: Some(info.buffer_size() * 3),
    })
}
```

**Encode side** (default computes input_bytes from dimensions):

```rust
fn estimated_cost(&self, width: u32, height: u32, format: PixelDescriptor) -> EncodeCost {
    let input_bytes = width as u64 * height as u64 * format.bytes_per_pixel() as u64;
    EncodeCost {
        input_bytes,
        pixel_count: width as u64 * height as u64,
        // AV1: ~6x input for transform + RDO + reference frames
        peak_memory: Some(input_bytes * 6),
    }
}
```

Callers validate costs against `ResourceLimits` using `check_decode_cost()` / `check_encode_cost()`. When `peak_memory` is `None`, these fall back to buffer size as a lower-bound estimate.

### Decode hints — `with_crop_hint()`, `with_scale_hint()`, `with_orientation_hint()`

These are optional hints on `DecodeJob`. The defaults are no-ops (return `self`). Override them when your codec can apply the transform cheaply during decode:

- **`with_crop_hint(x, y, w, h)`**: Request a region crop. JPEG codecs can crop on MCU boundaries; AV1 codecs can skip tiles outside the region. The actual crop may differ from the request — check `OutputInfo::crop_applied`.
- **`with_scale_hint(w, h)`**: Request prescaling. JPEG can decode at 1/2, 1/4, 1/8 resolution. JXL has resolution levels. The decoder picks the closest efficient resolution.
- **`with_orientation_hint(orientation)`**: Request that the decoder apply EXIF orientation during decode. If honored, `OutputInfo::orientation_applied` reflects the applied orientation and `width`/`height` reflect the rotated dimensions.

Codecs that don't support a hint simply ignore it (the default implementation returns `self`). `output_info()` must reflect whatever hints were actually applied.

### `with_limits(self, limits: ResourceLimits) -> Self`

Takes `ResourceLimits` by value (it's `Copy`). Codecs store limits and enforce what they can — not every codec supports every limit type. The limits your codec doesn't enforce are silently ignored; callers check `capabilities()` to know what's enforced.

This method appears on both config traits and both job traits. The job-level override takes precedence over the config-level setting.

### `with_stop()` and cooperative cancellation

`EncodeJob::with_stop()` and `DecodeJob::with_stop()` accept a `&'a dyn Stop` token. If your codec supports cancellation, periodically call `stop.check()` during encode/decode and return an error if it signals cancellation.

If cancellation isn't feasible for your codec (or for one side — encode vs decode), accept the token but don't check it, and set `encode_cancel: false` or `decode_cancel: false` in capabilities.

### `with_metadata()` and metadata handling

`EncodeJob::with_metadata()` receives a `MetadataView<'a>` with optional ICC, EXIF, and XMP data. Embed whatever your format supports. Metadata types that the format can't represent are silently skipped — but `capabilities()` must accurately reflect what gets embedded.

### Encoder trait — three paths

The `Encoder` trait provides three mutually exclusive usage paths:

1. **`encode(self, pixels)`** — one-shot, consumes the encoder
2. **`push_rows(&mut self, rows)` + `finish(self)`** — caller pushes rows incrementally
3. **`encode_from(self, source)`** — encoder pulls rows from a callback

Codecs that need full-frame data (e.g. AV1) may buffer internally for paths 2 and 3.

### FrameEncoder trait — three per-frame paths

The `FrameEncoder` trait provides three mutually exclusive per-frame paths:

1. **`push_frame(&mut self, pixels, duration_ms)`** — full-canvas frame at once
2. **`begin_frame()` + `push_rows()` + `end_frame()`** — build frame row-by-row
3. **`pull_frame(&mut self, duration_ms, source)`** — encoder pulls rows from callback

For sub-canvas frames with compositing, use **`push_encode_frame(&mut self, EncodeFrame)`** instead of `push_frame`. `EncodeFrame` carries `frame_rect` (canvas position), `blend` mode, and `disposal` method. Set the canvas dimensions with `EncodeJob::with_canvas_size()` before creating the frame encoder.

Call `finish(self)` after all frames are written.

### Decoder trait — three paths

The `Decoder` trait provides three options:

1. **`decode(self, data)`** — returns owned `DecodeOutput` (codec picks native format)
2. **`decode_into(self, data, dst)`** — decode into a caller-provided `PixelSliceMut`
3. **`decode_rows(self, data, sink)`** — decoder pushes rows to a callback

### FrameDecoder trait — three per-frame paths

The `FrameDecoder` trait provides three options per frame:

1. **`next_frame(&mut self)`** — pull a complete `DecodeFrame`
2. **`next_frame_into(&mut self, dst, prior_frame)`** — pull frame into caller buffer
3. **`next_frame_rows(&mut self, sink)`** — decoder pushes rows to callback

All return `None` when there are no more frames.

**`frame_count(&self)`** returns the number of frames if known without decoding. Default returns `None`. Override for formats where the container header contains a frame count.

**`next_frame_into`** takes a `prior_frame: Option<u32>` hint. When `Some(n)`, the caller's buffer already contains frame `n`'s composited result — the decoder can skip re-rendering the canvas from scratch. When `None`, the buffer contents are undefined. Codecs that don't use this hint can ignore it.

### Frame compositing metadata

`DecodeFrame` carries an `Arc<ImageInfo>` for container-level metadata (format, color space, ICC/EXIF/XMP, orientation) shared across all frames without per-frame duplication. Access via `info()`, `info_arc()`, `metadata()`, and `format()`.

`DecodeFrame` also carries compositing information for correct animation rendering:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `required_frame()` | `Option<u32>` | `None` | Prior frame needed for compositing. `None` = keyframe. |
| `blend()` | `FrameBlend` | `Source` | How to composite: `Source` (replace) or `Over` (alpha blend). |
| `disposal()` | `FrameDisposal` | `None` | Canvas cleanup: `None`, `RestoreBackground`, or `RestorePrevious`. |
| `frame_rect()` | `Option<[u32; 4]>` | `None` | Canvas region `[x, y, w, h]`. `None` = full canvas. |

These map directly to the semantics used by APNG, GIF, and WebP animations. AVIF animations don't use partial-canvas updates, so AVIF codecs leave all fields at defaults (every frame is a full-canvas keyframe with `Source` blend).

Builder methods: `with_required_frame()`, `with_blend()`, `with_disposal()`, `with_frame_rect()`.

### `encode_bgra8()` and `encode_bgrx8()` — default swizzle

The `EncoderConfig` trait provides default implementations that route through `PixelSlice::from()`. If your codec handles BGRA natively, override these on the config or encoder for zero-copy.

### `decode_into_*()` — zero-copy decode path

The `DecoderConfig` trait provides default implementations that route through `Decoder::decode_into()`. If your codec can decode directly into a caller-provided buffer, override `Decoder::decode_into()`. Callers should use `DecodeJob::output_info()` to determine the required buffer dimensions and pixel format.

### Error types

Each trait has an associated `type Error: core::error::Error + Send + Sync + 'static`. Each codec defines its own error type — there's no shared `CodecError`. This keeps error types precise and avoids forcing codecs into a one-size-fits-all error enum.

### `PixelData` variants

`PixelData` is a `#[non_exhaustive]` enum over `ImgVec<T>` for each pixel format:

- `Rgb8`, `Rgba8`, `Bgra8`, `Gray8` — 8-bit per channel
- `Rgb16`, `Rgba16`, `Gray16` — 16-bit per channel
- `GrayAlpha8`, `GrayAlpha16`, `GrayAlphaF32` — grayscale + alpha
- `RgbF32`, `RgbaF32`, `GrayF32` — 32-bit float

Return whichever variant your codec produces natively. `PixelData` is a pure data container — it holds pixel buffers but does not convert between formats. Use `descriptor()` to get the matching `PixelDescriptor` for any variant, and `as_rgb8()` / `as_rgba8()` etc. to borrow the data if it's already in the right format.

If you need a specific pixel format, use `Decoder::decode_into()` to request it from the codec (correct transfer function handling). For format-erased processing, convert to `PixelBuffer` with `PixelBuffer::from(pixel_data)` and work with raw byte rows.

### Checklist for a new codec

**Must implement** (required by trait definitions):

1. Config types (`MyEncoderConfig`, `MyDecoderConfig`) implementing `Clone + Send + Sync`
2. Job types with `'a` lifetime for borrowed data
3. Executor types (`MyEncoder`, `MyDecoder`, and optionally `MyFrameEncoder`, `MyFrameDecoder`)
4. `EncoderConfig`/`DecoderConfig` on config types with GATs: `type Job<'a> = MyEncodeJob<'a> where Self: 'a`
5. `fn format() -> ImageFormat` — the format this codec handles (static, known at the type level)
6. `fn capabilities() -> &'static CodecCapabilities` — honest feature flags (including `effort_range`, `quality_range`, `lossless`)
7. `fn supported_descriptors() -> &'static [PixelDescriptor]` — native pixel formats (hard guarantee: `decode_into`/`encode` must work without conversion for every listed descriptor)
8. `fn probe_header()` — O(header), never O(pixels)
9. `EncoderConfig::with_effort()`, `with_calibrated_quality()`, `with_lossless()` — map universal parameters to codec-specific settings (have defaults, override when supported)
10. `EncodeJob::with_stop()`, `with_metadata()`, `with_limits()`
11. `DecodeJob::with_stop()`, `with_limits()`
12. `DecodeJob::output_info()` — return `OutputInfo` reflecting current hints
13. `Encoder::encode()`, `push_rows()`/`finish()`, `encode_from()`
14. `Decoder::decode()`, `decode_into()`, `decode_rows()`
15. `FrameEncoder::push_frame()`, `begin_frame()`/`push_rows()`/`end_frame()`, `pull_frame()`, `finish()` — if animation supported
16. `FrameDecoder::next_frame()`, `next_frame_into()`, `next_frame_rows()` — if animation supported

**Should implement** (have defaults, but override when beneficial):

17. `probe_full()` — override when frame count or complete metadata requires a full parse
18. `with_crop_hint()`, `with_scale_hint()`, `with_orientation_hint()` on `DecodeJob` — honor decode hints when the codec can apply spatial transforms cheaply (JPEG MCU-aligned crop, JPEG 1/2/4/8 prescale, etc.)
19. `estimated_cost()` on `DecodeJob` — override to provide codec-specific `peak_memory` estimates (default derives from `output_info()`)
20. `estimated_cost()` on `EncodeJob` — override to provide codec-specific `peak_memory` estimates (default computes `input_bytes` from dimensions)
21. `frame_count()` on `FrameDecoder` — return frame count if known from container headers
22. `loop_count()` on `FrameDecoder` — return animation loop count if known
23. `with_loop_count()` on `FrameEncoder` — set animation loop count
24. `with_alpha_quality()` / `alpha_quality()` on `EncoderConfig` — independent alpha plane quality for codecs that support it (AVIF, WebP, JXL)
25. `with_canvas_size()` on `EncodeJob` — set animation canvas dimensions for compositing formats (GIF, APNG, WebP, JXL)
26. Populate `DecodeFrame` compositing fields (`with_required_frame()`, `with_blend()`, `with_disposal()`, `with_frame_rect()`) for animated formats with partial-canvas updates (GIF, APNG, WebP)
27. Honor `prior_frame` hint in `next_frame_into()` for efficient incremental animation compositing
28. Set `ImageInfo::cicp` on decode output when the format provides color description (AVIF, JXL, HEIF). Without CICP, `transfer_function()` returns `Unknown` and callers can't perform correct color-sensitive operations (resize, blend, blur)

## Pre-v0.1 Concerns

Open design questions that should be resolved before publishing. Adding required trait methods after v0.1 is a semver break, so the trait surface needs to be right.

### Metadata communication

**No encode-side contract for CICP vs ICC precedence.** The decode side documents "CICP takes precedence per AVIF/HEIF spec." On encode, if `MetadataView` contains both CICP and ICC, what should the codec do? Embed both when the format allows? Prefer one? The contract is silent.

**No individual metadata setters on `EncodeJob`.** The current trait only has `with_metadata()`. This works (via `MetadataView::none().with_icc(data).with_orientation(Rotate90)`) but is more verbose than separate `with_icc()`, `with_exif()`, `with_xmp()` methods.

### Extra layers and gain maps

**`GainMapMetadata` is defined** (ISO 21496-1), shared by JPEG UltraHDR, AVIF, and JXL. Codecs put `(PixelData, GainMapMetadata)` in `DecodeOutput::extras` via the type-erased `Any` escape hatch. This works but requires callers to know the concrete type to downcast.

**Gain map trait methods are deferred.** Only 3 of 6 codecs support gain maps (JPEG, AVIF, JXL). Adding `decode_gain_map()` / `encode_gain_map()` to the traits before proving the pattern across implementations risks getting the API wrong. Plan: prove via `extras` across 2-3 codecs, then promote to trait methods with defaults (semver-safe addition).

**Secondary images vary widely across formats.** Gain maps, depth maps, and alpha planes are all "secondary images" but differ in key properties:

| Property | Gain map | Depth map | Alpha plane |
|----------|----------|-----------|-------------|
| Resolution vs base | Different (typically 1/4 per axis) | Different | Must match (AVIF, WebP) |
| Pixel format | Grayscale | Monochrome | Monochrome |
| Needs own decode | Yes (separate codec invocation) | Yes | Format-dependent |
| Spatial relationship | Covers same extent, coarser grid | Same extent, coarser grid | Same pixel grid |

**Extra layers need equal streamability.** A gain map is itself an image — it needs the same decode paths (one-shot, row-level, into-buffer) as the primary image. Strip-based pipelines that apply a gain map need to stream both the base image and the gain map simultaneously. The current API has no mechanism for multi-layer streaming — a future version may add parallel decode paths.

### Streaming

**No incremental data feeding.** Every decode method takes `data: &[u8]` — the complete file must be in memory. There's no way to feed bytes incrementally from a network socket or streaming reader. This is a known v0.1 limitation. Adding it later means new trait methods (semver-safe with defaults) or a new `StreamingDataSource` trait. The question is whether v0.1's `&[u8]` constraint will cause real adoption pain.

**Strip pipeline orchestration is undefined.** The traits provide row-level push/pull for individual images. But composing codecs into a pipeline (decode → color convert → apply gain map → resize → encode) requires strip-height negotiation between stages with different natural strip sizes (JPEG: 8/16 rows, AV1: 64x64 tiles, PNG: 1 row). This orchestration belongs in a pipeline crate, not here — but the boundary between "codec contract" and "pipeline concern" needs a clear line.

**Animation streaming vs buffering.** The current `FrameEncoder` accepts frames one at a time (streamable). `FrameDecoder` returns frames one at a time (streamable). But the convenience `encode_animation_rgb8(&[TypedEncodeFrame])` buffers all frames in a slice, and there's no streaming counterpart on the trait. For large animations (animated AVIF, long GIF), the caller must drop to the `FrameEncoder` / `FrameDecoder` level manually. Is that acceptable, or should the trait provide iterator-based convenience methods?

### Animation

**~~Loop count not on traits.~~** Resolved: `FrameEncoder::with_loop_count()` and `FrameDecoder::loop_count()` are now on the traits with default no-op implementations. `total_duration_ms()` remains absent — computing it requires decoding all frames (can't know up front).

**~~Variable frame dimensions.~~** Resolved. `EncodeJob::with_canvas_size()` sets the canvas. `FrameEncoder::push_encode_frame()` accepts an `EncodeFrame` with `frame_rect`, `blend`, and `disposal` for sub-canvas frames. On the decode side, `ImageInfo` provides canvas dimensions and `DecodeFrame::frame_rect()` reports each frame's region. Pixel format is always consistent within a sequence across all formats.

### Pixel format

**No f32 animation encode convenience.** The trait has `encode_animation_rgb8` / `encode_animation_rgba8` / `encode_animation_rgb16` / `encode_animation_rgba16` but no f32 variants. This is intentional — the typed convenience methods don't track transfer function, and f32 data can be linear, PQ, HLG, or anything else. Callers needing f32 animation encoding use `FrameEncoder` directly with `PixelSlice::from()`.

### Naming and packaging

**~~Crate name.~~** Resolved: `zencodec-types`.

## License

Apache-2.0 OR MIT
