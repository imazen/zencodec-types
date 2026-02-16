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

**Image metadata** — `ImageInfo`, `ImageMetadata`, `OutputInfo`, `Cicp`, `ContentLightLevel`, `MasteringDisplay`, `Orientation`

**I/O types** — `EncodeOutput`, `DecodeOutput`, `DecodeFrame`, `EncodeFrame`, `TypedEncodeFrame`, `FrameBlend`, `FrameDisposal`

**Color management** — `ColorProfileSource`, `NamedProfile`

**Cost estimation** — `DecodeCost`, `EncodeCost`

**Resource management** — `ResourceLimits`, `LimitExceeded`, `ImageFormat` (magic-byte detection), `CodecCapabilities` (20 feature flags)

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

### Color profile source

`ImageInfo` and `ImageMetadata` carry raw color data (CICP codes, ICC bytes, HDR metadata). The `color_profile_source()` method returns a unified `ColorProfileSource` that consumers can pass directly to a CMS backend (e.g., moxcms):

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
2. **`next_frame_into(&mut self, dst, prior_frame)`** — pull frame into caller buffer
3. **`next_frame_rows(&mut self, sink)`** — decoder pushes rows to callback

All return `None` when there are no more frames.

**`frame_count(&self)`** returns the number of frames if known without decoding. Default returns `None`. Override for formats where the container header contains a frame count.

**`next_frame_into`** takes a `prior_frame: Option<u32>` hint. When `Some(n)`, the caller's buffer already contains frame `n`'s composited result — the decoder can skip re-rendering the canvas from scratch. When `None`, the buffer contents are undefined. Codecs that don't use this hint can ignore it.

### Frame compositing metadata

`DecodeFrame` carries compositing information for correct animation rendering:

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

Return whichever variant your codec produces natively. Callers use `into_rgb8()`, `into_rgba8()`, `into_gray8()`, etc. for conversion; the conversions handle all variant-to-target paths. The `descriptor()` method returns the matching `PixelDescriptor` for any variant.

The 16-to-8 bit conversions use `(v * 255 + 32768) >> 16` for proper rounding. All conversions assume sRGB; no linearization is performed.

For format-erased processing, convert to `PixelBuffer` with `PixelBuffer::from(pixel_data)` and work with raw byte rows instead of matching variants.

### Checklist for a new codec

**Must implement** (required by trait definitions):

1. Config types (`MyEncoderConfig`, `MyDecoderConfig`) implementing `Clone + Send + Sync`
2. Job types with `'a` lifetime for borrowed data
3. Executor types (`MyEncoder`, `MyDecoder`, and optionally `MyFrameEncoder`, `MyFrameDecoder`)
4. `EncoderConfig`/`DecoderConfig` on config types with GATs: `type Job<'a> = MyEncodeJob<'a> where Self: 'a`
5. `fn capabilities() -> &'static CodecCapabilities` — honest feature flags
6. `fn supported_descriptors() -> &'static [PixelDescriptor]` — native pixel formats (hard guarantee: `decode_into`/`encode` must work without conversion for every listed descriptor)
7. `fn probe_header()` — O(header), never O(pixels)
8. `EncodeJob::with_stop()`, `with_metadata()`, `with_limits()`
9. `DecodeJob::with_stop()`, `with_limits()`
10. `DecodeJob::output_info()` — return `OutputInfo` reflecting current hints
11. `Encoder::encode()`, `push_rows()`/`finish()`, `encode_from()`
12. `Decoder::decode()`, `decode_into()`, `decode_rows()`
13. `FrameEncoder::push_frame()`, `begin_frame()`/`push_rows()`/`end_frame()`, `pull_frame()`, `finish()` — if animation supported
14. `FrameDecoder::next_frame()`, `next_frame_into()`, `next_frame_rows()` — if animation supported

**Should implement** (have defaults, but override when beneficial):

15. `probe_full()` — override when frame count or complete metadata requires a full parse
16. `with_crop_hint()`, `with_scale_hint()`, `with_orientation_hint()` on `DecodeJob` — honor decode hints when the codec can apply spatial transforms cheaply (JPEG MCU-aligned crop, JPEG 1/2/4/8 prescale, etc.)
17. `estimated_cost()` on `DecodeJob` — override to provide codec-specific `peak_memory` estimates (default derives from `output_info()`)
18. `estimated_cost()` on `EncodeJob` — override to provide codec-specific `peak_memory` estimates (default computes `input_bytes` from dimensions)
19. `frame_count()` on `FrameDecoder` — return frame count if known from container headers
20. Populate `DecodeFrame` compositing fields (`with_required_frame()`, `with_blend()`, `with_disposal()`, `with_frame_rect()`) for animated formats with partial-canvas updates (GIF, APNG, WebP)
21. Honor `prior_frame` hint in `next_frame_into()` for efficient incremental animation compositing

## Pre-v0.1 Concerns

Open design questions that should be resolved before publishing. Adding required trait methods after v0.1 is a semver break, so the trait surface needs to be right.

### Metadata communication

**Orientation belongs in `ImageMetadata`.** Currently `ImageInfo` carries orientation but `ImageMetadata` does not. When a codec extracts and strips orientation from EXIF during decode, the roundtrip path (`decoded.metadata()` → `job.with_metadata(&meta)`) loses it. Orientation should be a field on `ImageMetadata` — and it must be mutable, because callers frequently resolve or remove orientation during processing (apply rotation, then set to Normal before re-encoding).

**No encode-side contract for CICP vs ICC precedence.** The decode side documents "CICP takes precedence per AVIF/HEIF spec." On encode, if `ImageMetadata` contains both CICP and ICC, what should the codec do? Embed both when the format allows? Prefer one? The contract is silent.

**`DecodeFrame` carries no metadata.** Individual animation frames have pixels but no `ImageInfo`, CICP, or bit depth. Container-level metadata covers most cases, but the frame has no way to communicate per-frame color info. Is that acceptable, or should `DecodeFrame` carry at least a reference to container-level `ImageInfo`?

**No individual metadata setters on `EncodeJob`.** The api-design.md shows `with_icc()`, `with_exif()`, `with_xmp()` as separate methods. The current trait only has `with_metadata()`. This works (via `ImageMetadata::none().with_icc(data)`) but means the trait has no way to set orientation — it only flows through the `ImageMetadata` struct, which currently lacks that field.

### Extra layers and gain maps

**Extra layers come in multiple forms.** JPEG UltraHDR embeds a gain map as a secondary JPEG in MPF. AVIF stores it as a `tmap` item with an AV1-encoded gain map. JXL has its own gain map mechanism. A unified `GainMapImage` type needs to abstract over these container differences while preserving the metadata each format requires (ISO 21496-1 `GainMapMetadata`, base/alternate CICP, HDR metadata).

**Extra layers need equal streamability.** A gain map is itself an image — it needs the same decode paths (one-shot, row-level, into-buffer) as the primary image. Strip-based pipelines that apply a gain map need to stream both the base image and the gain map simultaneously, with the gain map reader producing rows in lockstep. The current API has no mechanism for multi-layer streaming — `DecodeOutput` returns one `PixelData`, and extra layers are behind a type-erased `Any` escape hatch.

**`DecodeOutput::extras` uses type-erased `Any`.** This is the current escape hatch for format-specific data (gain maps, MPF secondary images). It works but means callers must know the concrete type to downcast. For gain maps specifically, should there be a structured path (`decode_gain_map` / `encode_gain_map` on the traits) instead of or in addition to the `Any` escape hatch?

**Timing: gain map trait methods before or after v0.1?** Adding them before v0.1 (as required methods) forces every codec to handle them immediately. Adding them after v0.1 (as default-impl'd methods returning `Ok(None)` or `Err(Unsupported)`) avoids the semver break but hides missing functionality behind defaults. Only JPEG, AVIF, and JXL support gain maps — GIF, PNG, and WebP do not.

### Streaming

**No incremental data feeding.** Every decode method takes `data: &[u8]` — the complete file must be in memory. There's no way to feed bytes incrementally from a network socket or streaming reader. This is a known v0.1 limitation. Adding it later means new trait methods (semver-safe with defaults) or a new `StreamingDataSource` trait. The question is whether v0.1's `&[u8]` constraint will cause real adoption pain.

**Strip pipeline orchestration is undefined.** The traits provide row-level push/pull for individual images. But composing codecs into a pipeline (decode → color convert → apply gain map → resize → encode) requires strip-height negotiation between stages with different natural strip sizes (JPEG: 8/16 rows, AV1: 64x64 tiles, PNG: 1 row). This orchestration belongs in a pipeline crate, not here — but the boundary between "codec contract" and "pipeline concern" needs a clear line.

**Animation streaming vs buffering.** The current `FrameEncoder` accepts frames one at a time (streamable). `FrameDecoder` returns frames one at a time (streamable). But the convenience `encode_animation_rgb8(&[TypedEncodeFrame])` buffers all frames in a slice, and there's no streaming counterpart on the trait. For large animations (animated AVIF, long GIF), the caller must drop to the `FrameEncoder` / `FrameDecoder` level manually. Is that acceptable, or should the trait provide iterator-based convenience methods?

### Animation

**Loop count and duration not on traits.** `FrameEncoder` has no `with_loop_count()`. `FrameDecoder` has no `loop_count()` or `total_duration_ms()`. These are currently format-specific methods on concrete types. Should the traits carry them?

**Variable frame dimensions.** The API assumes all animation frames share one pixel format. `DecodeFrame` has `frame_rect` for sub-canvas regions, but the trait doesn't negotiate canvas dimensions upfront. For formats like GIF and APNG where frames can have different dimensions than the canvas, the canvas size comes from `ImageInfo` on probe — but `FrameEncoder` has no `with_canvas_size()`.

### Pixel format

**Quality/effort/lossless not on traits.** The api-design.md puts `with_quality()`, `with_effort()`, `with_lossless()` on the `Encoding` trait. The implementation leaves them on concrete types only. Putting them on the trait enables `fn compress(cfg: &impl EncoderConfig, ...)` to set quality generically. Leaving them off keeps the trait minimal and avoids forcing all codecs to accept parameters they might not support (GIF has no meaningful "quality 0-100" mapping). Decided against — but worth reconsidering if generic pipelines need it.

**HDR encode paths.** The trait has `encode_rgb16` / `encode_rgba16` but the f32 → u16 → f32 roundtrip is lossy for HDR content. PQ-encoded u16 has 12-bit effective precision. For HDR workflows that stay in f32 linear light, the f32 encode methods exist — but there's no f32 animation encode convenience (`encode_animation_rgb_f32`). Is that a gap?

### Naming and packaging

**Crate name.** Currently `zencodec-types` (repo: `zencodec-types-api`). The api-design.md uses `zencodec`. Options: `zencodec`, `zencodec-types`, `zencodec-api`, `codec-api`. The name signals whether this is zen*-specific or potentially reusable by other codec families.

## License

Apache-2.0 OR MIT
