# Feedback Log

## 2026-02-17
- User requested implementation of full feature support plan: UnsupportedOperation enum, HasUnsupportedOperation trait, expanded CodecCapabilities, format() on traits, with_alpha_quality/alpha_quality, animation loop count, BGRX8_SRGB constant
- User requested research on no_std I/O abstraction crates: embedded-io, no-std-io, musli-zerocopy, bytes, bare-io, core2, and core::io stabilization status

## 2026-02-18
- User: transfer functions are too slow unoptimized and wrong is worse than gone for zencodec-types. Decision: don't add transfer function conversion to types crate, fix rounding bug, make docs honest about raw numeric conversions
- User requested research on how animated image formats handle variable frame dimensions, canvas sizes, per-frame pixel formats, and frame positioning for GIF, APNG, WebP, AVIF/HEIF, and JPEG XL. Also Rust library APIs for animation.
- User requested deep research on gain maps and secondary/auxiliary images across formats: JPEG UltraHDR (ISO 21496-1), AVIF/HEIF tmap, JXL extra channels/gain maps, depth maps, alpha planes. Concrete API shapes from libheif, libjxl, libultrahdr, libavif.
- User requested fix of zengif codec to compile against updated zencodec-types API: add format() to EncoderConfig/DecoderConfig impls, replace removed DecodeOutput conversion methods with local helpers.
- User requested implementation of cross-format image pipeline in zencodecs: Pipeline builder with QualityPreset, MetadataPolicy, layout constraints (fit/within/crop/pad), EXIF auto-orientation, lossless passthrough, zenresize integration. Feature-gated behind `pipeline`.
- User requested fix of broken trait re-exports so zencodecs compiles: renamed *Encoding/*Decoding to *EncoderConfig/*DecoderConfig, restructured adapter code to use correct trait method hierarchy (config→job→encoder/decoder).
