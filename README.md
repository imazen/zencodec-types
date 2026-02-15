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

**Traits** — `EncoderConfig`, `EncodeJob`, `Encoder`, `FrameEncoder`, `DecoderConfig`, `DecodeJob`, `Decoder`, `FrameDecoder`

**Pixel data** — `PixelData` (typed enum over `ImgVec<T>`), `GrayAlpha<T>`

**Opaque pixel buffers** — `PixelBuffer`, `PixelSlice`, `PixelSliceMut`, `PixelDescriptor`, `ChannelType`, `ChannelLayout`, `AlphaMode`, `TransferFunction`, `BufferError`

**Image metadata** — `ImageInfo`, `ImageMetadata`, `Cicp`, `ContentLightLevel`, `MasteringDisplay`, `Orientation`

**I/O types** — `EncodeOutput`, `DecodeOutput`, `DecodeFrame`, `EncodeFrame`, `TypedEncodeFrame`

**Discovery** — `ImageFormat` (magic-byte detection), `CodecCapabilities` (17 feature flags), `ResourceLimits`

**Re-exports** — `imgref`, `rgb`, `enough` (for `Stop`/`Unstoppable`)

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

**Config** types (`EncoderConfig`, `DecoderConfig`) are reusable, `Clone + Send + Sync`, and have no lifetimes. They hold format-specific settings (quality, effort, lossless mode) as methods on the concrete type — the traits don't touch those. You can store configs in structs, share them across threads, and create multiple jobs from one config.

**Job** types (`EncodeJob`, `DecodeJob`) are per-operation. They borrow temporary data (stop tokens, metadata, resource limits) via a `'a` lifetime and produce an executor.

**Executor** types (`Encoder`/`FrameEncoder`, `Decoder`/`FrameDecoder`) run the actual encode/decode. Single-image executors are consumed by one-shot methods. Animation executors are mutable and produce/consume frames iteratively.

```rust
use zenjpeg::{JpegEncoderConfig, JpegDecoderConfig};
use zencodec_types::{EncoderConfig, DecoderConfig, ResourceLimits};

// Config: set format-specific options on the concrete type
let config = JpegEncoderConfig::new()
    .with_quality(85.0)
    .with_limits(ResourceLimits::none().with_max_pixels(100_000_000));

// One-shot convenience: encode directly from config
let output = config.encode_rgb8(img.as_ref())?;

// Full pipeline: config → job → encoder → encode
let output = config.job()
    .with_metadata(&metadata)
    .with_stop(&stop_token)
    .encoder()
    .encode(PixelSlice::from(img.as_ref()))?;
```

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
```

### Decode paths

```rust
// One-shot decode (typed convenience)
let output = config.decode(data)?;

// Decode into caller buffer
let mut buf = PixelBuffer::new(w, h, PixelDescriptor::RGBA8_SRGB)?;
config.job().decoder().decode_into(data, buf.as_mut_slice())?;

// Row-level callback: decoder pushes rows as they're available
config.job().decoder().decode_rows(data, &mut |row_idx, row| {
    process_row(row_idx, row);
})?;

// Animation: pull complete frames
let mut dec = config.job().frame_decoder(data)?;
while let Some(frame) = dec.next_frame()? {
    process(frame);
}

// Animation: pull frames into caller buffer
let mut dec = config.job().frame_decoder(data)?;
while let Some(info) = dec.next_frame_into(buf.as_mut_slice())? {
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

**`PixelDescriptor`** packs `ChannelType` (U8/U16/F32), `ChannelLayout` (Gray/GrayAlpha/Rgb/Rgba/Bgra), `AlphaMode` (None/Straight/Premultiplied), and `TransferFunction` (Linear/Srgb/Bt709/Pq/Hlg) into 4 bytes. Named constants like `PixelDescriptor::RGB8_SRGB` cover the 13 standard formats.

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

### Transfer function conventions

- **u8 / u16**: gamma-encoded (typically sRGB). u16 uses the full 0-65535 range.
- **f32**: linear light (gamma = 1.0).

The actual transfer function is recorded in `PixelDescriptor::transfer` and in `ImageInfo::cicp`. Use `TransferFunction::from_cicp()` to map CICP transfer characteristic codes to the enum.

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

### `decode_info()` — predict output dimensions

`decode_info()` defaults to `probe_header()`, which is correct when your config doesn't transform dimensions. Override it if your codec applies scaling, orientation, or other transforms that change the output size. Callers use this to allocate buffers for `decode_into_*` methods.

### `with_limits(self, limits: ResourceLimits) -> Self`

Takes `ResourceLimits` by value (it's `Copy`). Codecs store limits and enforce what they can — not every codec supports every limit type. The limits your codec doesn't enforce are silently ignored; callers check `capabilities()` to know what's enforced.

This method appears on both config traits and both job traits. The job-level override takes precedence over the config-level setting.

### `with_stop()` and cooperative cancellation

`EncodeJob::with_stop()` and `DecodeJob::with_stop()` accept a `&'a dyn Stop` token. If your codec supports cancellation, periodically call `stop.check()` during encode/decode and return an error if it signals cancellation.

If cancellation isn't feasible for your codec (or for one side — encode vs decode), accept the token but don't check it, and set `encode_cancel: false` or `decode_cancel: false` in capabilities.

### `with_metadata()` and metadata handling

`EncodeJob::with_metadata()` receives an `ImageMetadata<'a>` with optional ICC, EXIF, and XMP data. Embed whatever your format supports. Metadata types that the format can't represent are silently skipped — but `capabilities()` must accurately reflect what gets embedded.

### Encoder trait — three paths

The `Encoder` trait provides three mutually exclusive usage paths:

1. **`encode(self, pixels)`** — one-shot, consumes the encoder
2. **`push_rows(&mut self, rows)` + `finish(self)`** — caller pushes rows incrementally
3. **`encode_from(self, source)`** — encoder pulls rows from a callback

Codecs that need full-frame data (e.g. AV1) may buffer internally for paths 2 and 3.

### FrameEncoder trait — three per-frame paths

The `FrameEncoder` trait provides three mutually exclusive per-frame paths:

1. **`push_frame(&mut self, pixels, duration_ms)`** — complete frame at once
2. **`begin_frame()` + `push_rows()` + `end_frame()`** — build frame row-by-row
3. **`pull_frame(&mut self, duration_ms, source)`** — encoder pulls rows from callback

Call `finish(self)` after all frames are written.

### Decoder trait — three paths

The `Decoder` trait provides three options:

1. **`decode(self, data)`** — returns owned `DecodeOutput` (codec picks native format)
2. **`decode_into(self, data, dst)`** — decode into a caller-provided `PixelSliceMut`
3. **`decode_rows(self, data, sink)`** — decoder pushes rows to a callback

### FrameDecoder trait — three per-frame paths

The `FrameDecoder` trait provides three options per frame:

1. **`next_frame(&mut self)`** — pull a complete `DecodeFrame`
2. **`next_frame_into(&mut self, dst)`** — pull frame into caller buffer
3. **`next_frame_rows(&mut self, sink)`** — decoder pushes rows to callback

All return `None` when there are no more frames.

### `encode_bgra8()` and `encode_bgrx8()` — default swizzle

The `EncoderConfig` trait provides default implementations that route through `PixelSlice::from()`. If your codec handles BGRA natively, override these on the config or encoder for zero-copy.

### `decode_into_*()` — zero-copy decode path

The `DecoderConfig` trait provides default implementations that route through `Decoder::decode_into()`. If your codec can decode directly into a caller-provided buffer, override `Decoder::decode_into()`. Callers should use `decode_info()` to determine the required buffer dimensions.

### Error types

Each trait has an associated `type Error: core::error::Error + Send + Sync + 'static`. Each codec defines its own error type — there's no shared `CodecError`. This keeps error types precise and avoids forcing codecs into a one-size-fits-all error enum.

### `PixelData` variants

`PixelData` is a `#[non_exhaustive]` enum over `ImgVec<T>` for each pixel format:

- `Rgb8`, `Rgba8`, `Bgra8`, `Gray8` — 8-bit per channel
- `Rgb16`, `Rgba16`, `Gray16` — 16-bit per channel
- `GrayAlpha8`, `GrayAlpha16`, `GrayAlphaF32` — grayscale + alpha
- `RgbF32`, `RgbaF32`, `GrayF32` — 32-bit float

Return whichever variant your codec produces natively. Callers use `into_rgb8()`, `into_rgba8()`, `into_gray8()`, etc. for conversion; the conversions handle all variant-to-target paths. The `descriptor()` method returns the matching `PixelDescriptor` for any variant.

The 16-to-8 bit conversions use `(v * 255 + 32768) >> 16` for proper rounding. All conversions assume sRGB; no linearization is performed.

For format-erased processing, convert to `PixelBuffer` with `PixelBuffer::from(pixel_data)` and work with raw byte rows instead of matching variants.

### Checklist for a new codec

1. Define your config types (e.g., `MyEncoderConfig`, `MyDecoderConfig`) implementing `Clone + Send + Sync`
2. Define your job types with a `'a` lifetime for borrowed data
3. Define your executor types (`MyEncoder`, `MyFrameEncoder`, `MyDecoder`, `MyFrameDecoder`)
4. Implement `EncoderConfig`/`DecoderConfig` on config types with GATs: `type Job<'a> = MyEncodeJob<'a> where Self: 'a`
5. Define `static` `CodecCapabilities` for encode and decode (they can differ)
6. Implement `probe_header()` — must be O(header). Override `probe_full()` only if needed.
7. Implement `with_limits()` on all four traits (config and job level)
8. Implement `with_stop()` and `with_metadata()` on job types
9. Implement `Encoder::encode()` and `Decoder::decode()` at minimum
10. Implement `Encoder::push_rows()`/`finish()` and `encode_from()` for incremental encode
11. Implement `FrameEncoder`/`FrameDecoder` if your format supports animation
12. Override `decode_info()` if your config transforms output dimensions

## License

Apache-2.0 OR MIT
