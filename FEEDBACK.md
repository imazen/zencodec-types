# Feedback Log

## 2026-02-17
- User requested implementation of full feature support plan: UnsupportedOperation enum, HasUnsupportedOperation trait, expanded CodecCapabilities, format() on traits, with_alpha_quality/alpha_quality, animation loop count, BGRX8_SRGB constant
- User requested research on no_std I/O abstraction crates: embedded-io, no-std-io, musli-zerocopy, bytes, bare-io, core2, and core::io stabilization status

## 2026-02-18
- User: transfer functions are too slow unoptimized and wrong is worse than gone for zencodec-types. Decision: don't add transfer function conversion to types crate, fix rounding bug, make docs honest about raw numeric conversions
