# zencodec-types

Shared traits and types for the zen\* image codec family.

This crate defines the common interface that all zen\* codecs implement. It contains no codec logic — just traits, types, and pixel format conversions. `no_std` compatible (requires `alloc`), `forbid(unsafe_code)`.

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

**Traits** — `Encoding`, `EncodingJob`, `Decoding`, `DecodingJob`

**Pixel data** — `PixelData` (typed enum over `ImgVec<T>`), `GrayAlpha<T>`

**Opaque pixel buffers** — `PixelBuffer`, `PixelSlice`, `PixelSliceMut`, `PixelDescriptor`, `ChannelType`, `ChannelLayout`, `AlphaMode`, `TransferFunction`, `BufferError`

**Image metadata** — `ImageInfo`, `ImageMetadata`, `Cicp`, `ContentLightLevel`, `MasteringDisplay`, `Orientation`

**I/O types** — `EncodeOutput`, `DecodeOutput`, `DecodeFrame`, `EncodeFrame`

**Discovery** — `ImageFormat` (magic-byte detection), `CodecCapabilities` (17 feature flags), `ResourceLimits`

**Re-exports** — `imgref`, `rgb`, `enough` (for `Stop`/`Unstoppable`)

## Architecture: Config/Job pattern

Codecs use a two-level pattern: **Config** types implement `Encoding`/`Decoding`, and **Job** types implement `EncodingJob`/`DecodingJob`.

Config types are reusable, `Clone + Send + Sync`, and have no lifetimes. They hold format-specific settings (quality, effort, lossless mode) as methods on the concrete type — the trait doesn't touch those. You can store configs in structs, share them across threads, and create multiple jobs from one config.

Job types are per-operation. They borrow temporary data (stop tokens, metadata) via the `'a` lifetime and are consumed by terminal encode/decode methods.

```rust
use zenjpeg::{JpegEncoder, JpegDecoder};
use zencodec_types::{Encoding, Decoding, ResourceLimits};

// Config: set format-specific options on the concrete type
let encoder = JpegEncoder::new()
    .with_quality(85.0)
    .with_limits(ResourceLimits::none().with_max_pixels(100_000_000));

// Job: attach per-operation data, then execute
let output = encoder.job()
    .with_metadata(&metadata)
    .with_stop(&stop_token)
    .encode_rgb8(img.as_ref())?;
```

## Opaque pixel buffers

`PixelData` is a 13-variant enum — every consumer must match all 13 arms to do anything generic. The `buffer` module provides an alternative: format-erased buffers tagged with a 4-byte `PixelDescriptor`.

**`PixelDescriptor`** packs `ChannelType` (U8/U16/F32), `ChannelLayout` (Gray/GrayAlpha/Rgb/Rgba/Bgra), `AlphaMode` (None/Straight/Premultiplied), and `TransferFunction` (Linear/Srgb/Bt709/Pq/Hlg) into 4 bytes. Named constants like `PixelDescriptor::RGB8_SRGB` cover the 13 standard formats.

**`PixelBuffer`** is an owned `Vec<u8>` with dimensions, stride, and descriptor. Allocate with `new()`, wrap a pool vec with `from_vec()`, recover it with `into_vec()`. Supports row access, sub-row slicing, and zero-copy crop views.

**`PixelSlice<'a>`** / **`PixelSliceMut<'a>`** are borrowed views with the same row/crop API. Zero-copy `From<ImgRef<T>>` impls exist for all 10 rgb-crate pixel types.

Conversions between `PixelData` and `PixelBuffer` always copy (no `unsafe` needed):

```rust
use zencodec_types::{PixelBuffer, PixelData, PixelDescriptor};

// PixelData → PixelBuffer
let buf = PixelBuffer::from(pixel_data);

// PixelBuffer → PixelData
let data = PixelData::try_from(buf)?;

// ImgRef → PixelSlice (zero-copy)
let slice = PixelSlice::from(img.as_ref());
```

### Transfer function conventions

- **u8 / u16**: gamma-encoded (typically sRGB). u16 uses the full 0-65535 range.
- **f32**: linear light (gamma = 1.0).

The actual transfer function is recorded in `PixelDescriptor::transfer` and in `ImageInfo::cicp`. Use `TransferFunction::from_cicp()` to map CICP transfer characteristic codes to the enum.

## Implementor reference

This section documents the contract that codec implementations must satisfy. If you're writing a new zen\* codec, read this carefully.

### Required trait bounds

Both `Encoding` and `Decoding` require `Sized + Clone + Send + Sync`. Your config types must be safe to share across threads. Job types do not have these bounds (they borrow `&'a` data from config and per-call arguments).

### `capabilities()` — declare what you support

Both traits require a `fn capabilities() -> &'static CodecCapabilities` method. This returns a static reference describing what the codec actually supports.

```rust
use zencodec_types::CodecCapabilities;

static ENCODE_CAPS: CodecCapabilities = CodecCapabilities::new()
    .with_encode_icc(true)
    .with_encode_exif(true)
    .with_encode_cancel(true)
    .with_cheap_probe(true);

impl Encoding for MyEncoder {
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

### `decode_info()` — predict output dimensions

`decode_info()` defaults to `probe_header()`, which is correct when your config doesn't transform dimensions. Override it if your codec applies scaling, orientation, or other transforms that change the output size. Callers use this to allocate buffers for `decode_into_*` methods.

### `with_limits(self, limits: ResourceLimits) -> Self`

Takes `ResourceLimits` by value (it's `Copy`). Codecs store limits and enforce what they can — not every codec supports every limit type. The limits your codec doesn't enforce are silently ignored; callers check `capabilities()` to know what's enforced.

This method appears on all four traits: `Encoding`, `Decoding`, `EncodingJob`, and `DecodingJob`. The job-level override takes precedence over the config-level setting.

### `with_stop()` and cooperative cancellation

`EncodingJob::with_stop()` and `DecodingJob::with_stop()` accept a `&'a dyn Stop` token. If your codec supports cancellation, periodically call `stop.check()` during encode/decode and return an error if it signals cancellation.

If cancellation isn't feasible for your codec (or for one side — encode vs decode), accept the token but don't check it, and set `encode_cancel: false` or `decode_cancel: false` in capabilities.

### `with_metadata()` and metadata handling

`EncodingJob::with_metadata()` receives an `ImageMetadata<'a>` with optional ICC, EXIF, and XMP data. Embed whatever your format supports. Metadata types that the format can't represent are silently skipped — but `capabilities()` must accurately reflect what gets embedded.

### `encode_bgra8()` and `encode_bgrx8()` — default swizzle

The trait provides default implementations that swizzle BGRA→RGBA and BGRX→RGB, then delegate to `encode_rgba8()` / `encode_rgb8()`. If your codec handles BGRA natively (common with platform APIs), override these for zero-copy.

### `decode_into_*()` — zero-copy decode path

The trait provides default implementations that decode, convert, and copy row-by-row. If your codec can decode directly into a caller-provided buffer, override these methods. Callers should use `decode_info()` to determine the required buffer dimensions.

### Error types

Each trait has an associated `type Error: core::error::Error + Send + Sync + 'static`. Each codec defines its own error type — there's no shared `CodecError`. This keeps error types precise and avoids forcing codecs into a one-size-fits-all error enum.

### `PixelData` variants

`PixelData` is a `#[non_exhaustive]` enum over `ImgVec<T>` for each pixel format:

- `Rgb8`, `Rgba8`, `Bgra8`, `Gray8` — 8-bit per channel
- `Rgb16`, `Rgba16`, `Gray16` — 16-bit per channel
- `GrayAlpha8`, `GrayAlpha16`, `GrayAlphaF32` — grayscale + alpha
- `RgbF32`, `RgbaF32`, `GrayF32` — 32-bit float

Return whichever variant your codec produces natively. Callers use `into_rgb8()`, `into_rgba8()`, `into_gray8()`, etc. for conversion; the conversions handle all variant→target paths. The `descriptor()` method returns the matching `PixelDescriptor` for any variant.

The 16→8 bit conversions use `(v * 255 + 32768) >> 16` for proper rounding. All conversions assume sRGB; no linearization is performed.

For format-erased processing, convert to `PixelBuffer` with `PixelBuffer::from(pixel_data)` and work with raw byte rows instead of matching variants.

### Checklist for a new codec

1. Define your config types (e.g., `MyEncoder`, `MyDecoder`) implementing `Clone + Send + Sync`
2. Define your job types with a `'a` lifetime for borrowed data
3. Implement `Encoding`/`Decoding` on config types with GATs: `type Job<'a> = MyEncodeJob<'a> where Self: 'a`
4. Define `static` `CodecCapabilities` for encode and decode (they can differ)
5. Implement `probe_header()` — must be O(header). Override `probe_full()` only if needed.
6. Implement `with_limits()` on all four traits (config and job level)
7. Implement `with_stop()` and `with_metadata()` on job types
8. Implement `encode_rgb8()`, `encode_rgba8()`, `encode_gray8()` at minimum
9. Implement `decode()` returning the most natural `PixelData` variant for your format
10. Override `encode_bgra8()`/`encode_bgrx8()` if your codec handles BGRA natively
11. Override `decode_into_*()` if your codec can write directly to a caller buffer
12. Override `decode_info()` if your config transforms output dimensions

## License

Apache-2.0 OR MIT
