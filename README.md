# zencodec ![ci](https://img.shields.io/github/actions/workflow/status/imazen/zencodec/ci.yml?branch=main&style=flat-square&label=CI) ![crates.io](https://img.shields.io/crates/v/zencodec?style=flat-square) ![docs.rs](https://img.shields.io/docsrs/zencodec?style=flat-square) ![msrv](https://img.shields.io/badge/MSRV-1.93-blue?style=flat-square) ![license](https://img.shields.io/crates/l/zencodec?style=flat-square)

zencodec is the shared trait crate that defines the common API for all zen\* image codecs.

It contains no codec logic — just traits, types, and format negotiation helpers. `no_std` compatible (requires `alloc`), `forbid(unsafe_code)`.

Import as `zencodec` — use `zencodec::encode`, `zencodec::decode`, etc.

## Crates in the zen\* family

| Crate | Format | Repo |
|-------|--------|------|
| `zenjpeg` | JPEG | [imazen/zenjpeg](https://github.com/imazen/zenjpeg) |
| `zenwebp` | WebP | [imazen/zenwebp](https://github.com/imazen/zenwebp) |
| `zenpng` | PNG | [imazen/zenpng](https://github.com/imazen/zenpng) |
| `zengif` | GIF | [imazen/zengif](https://github.com/imazen/zengif) |
| `zenavif` | AVIF | [imazen/zenavif](https://github.com/imazen/zenavif) |
| `zenjxl` | JPEG XL | [imazen/zenjxl](https://github.com/imazen/zenjxl) |
| `zenbitmaps` | PNM/BMP/Farbfeld | [imazen/zenbitmaps](https://github.com/imazen/zenbitmaps) |
| `heic` | HEIC/HEIF | [imazen/heic](https://github.com/imazen/heic) |
| `zentiff` | TIFF (experimental) | [imazen/zentiff](https://github.com/imazen/zentiff) |
| `zenpdf` | PDF (experimental) | [imazen/zenpdf](https://github.com/imazen/zenpdf) |

## Architecture

Every codec follows a three-layer pattern:

```text
Config     →  reusable, Clone + Send + Sync, 'static — consumed by job()
Job        →  per-operation, owns config + stop token + limits + metadata
Executor   →  borrows pixel data or file bytes, consumes self to produce output
```

```text
ENCODE:  EncoderConfig → EncodeJob → Encoder / AnimationFrameEncoder
DECODE:  DecoderConfig → DecodeJob<'a> → Decode / StreamingDecode / AnimationFrameDecoder
```

Config lives in a struct and gets shared across threads. A web server keeps one `JpegEncoderConfig` at quality 85 for all requests and clones it per-request. Calling `job()` consumes the config — clone first if you need it again. Job owns its config, cancellation token, resource limits, and metadata. Executor borrows pixels or bytes and consumes itself to produce output.

Each layer also has object-safe `Dyn*` variants for codec-agnostic dispatch:

```text
DynEncoderConfig → DynEncodeJob → DynEncoder / DynAnimationFrameEncoder
DynDecoderConfig → DynDecodeJob → DynDecoder / DynStreamingDecoder / DynAnimationFrameDecoder
```

Blanket impls generate the dyn API automatically — codec authors implement the generic traits and get dyn dispatch for free.

## Quick Example

```rust,ignore
use std::borrow::Cow;
use zenjpeg::{JpegEncoderConfig, JpegDecoderConfig};
use zencodec::encode::{EncoderConfig, EncodeJob, Encoder};
use zencodec::decode::{DecoderConfig, DecodeJob, Decode};

// Encode
let config = JpegEncoderConfig::new().with_generic_quality(85.0);
// (assuming pixels: PixelSlice from your pipeline)
let output = config.job().encoder()?.encode(pixels.as_slice())?;
let jpeg_bytes = output.into_vec();

// Decode
let config = JpegDecoderConfig::new();
let decoded = config.job().decoder(Cow::Borrowed(&jpeg_bytes), &[])?.decode()?;
let pixels = decoded.into_buffer();
```

## Key Design Decisions

**Color management is not the codec's job.** Decoders return native pixels with ICC/CICP metadata. Encoders accept pixels as-is and embed the provided metadata. The caller handles CMS transforms.

**Format negotiation over conversion.** Decoders take a ranked `&[PixelDescriptor]` preference list and pick the first they can produce without lossy conversion. Pass `&[]` for native format.

**Capabilities over try/catch.** Codecs declare their capabilities as const `EncodeCapabilities` / `DecodeCapabilities` structs. Check before calling instead of catching `UnsupportedOperation` errors.

**Pixel types from `zenpixels`.** All pixel interchange types (`PixelSlice`, `PixelBuffer`, `PixelDescriptor`, etc.) are defined in the `zenpixels` crate. All zen\* crates depend on `zenpixels` directly.

## What's in this crate

| Module | Contents |
|--------|----------|
| `zencodec::encode` | `EncoderConfig`, `EncodeJob`, `Encoder`, `AnimationFrameEncoder`, `EncodeOutput`, `EncodeCapabilities`, `EncodePolicy`, `best_encode_format`, dyn dispatch traits (`DynEncoderConfig`, `DynEncodeJob`, `DynEncoder`, `DynAnimationFrameEncoder`) |
| `zencodec::decode` | `DecoderConfig`, `DecodeJob`, `Decode`, `StreamingDecode`, `AnimationFrameDecoder`, `DecodeOutput`, `DecodeCapabilities`, `DecodePolicy`, `DecodeRowSink`, `SinkError`, `OutputInfo`, `SourceEncodingDetails`, `negotiate_pixel_format`, `is_format_available`, dyn dispatch traits (`DynDecoderConfig`, `DynDecodeJob`, `DynDecoder`, `DynStreamingDecoder`, `DynAnimationFrameDecoder`) |
| `zencodec::gainmap` | `GainMapInfo`, `GainMapParams`, `GainMapChannel`, `GainMapDirection`, `GainMapPresence` — cross-codec gain map types (ISO 21496-1) |
| `zencodec::helpers` | Codec implementation helpers (not consumer API) — shared boilerplate for trait implementors |
| root | `ImageFormat`, `ImageFormatDefinition`, `ImageFormatRegistry` (format detection via `ImageFormatRegistry::detect()`), `ImageInfo`, `Metadata`, `Orientation`, `OrientationHint`, `ResourceLimits`, `LimitExceeded`, `ThreadingPolicy`, `UnsupportedOperation`, `CodecErrorExt`, `find_cause`, `Unsupported`, `Extensions`, `AnimationFrame`, `OwnedAnimationFrame`, `Cicp`, `ContentLightLevel`, `MasteringDisplay`, `StopToken`, `Unstoppable` |

zencodec has no feature flags. The full API is always available.

## Limitations

- Contains no codec logic — traits, types, and format detection only.
- `ImageFormat` enum is not extensible at runtime (the `Custom` variant requires a `&'static` definition).
- Always `no_std` + `alloc` (no `std` feature gate).

## MSRV

Rust 1.93+, 2024 edition.

## License

Apache-2.0 OR MIT
