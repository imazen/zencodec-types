# zencodec-types

Shared traits and types for zen* image codecs.

See `/home/lilith/work/zencodecs/api-design.md` for the full API design.

## Purpose

Tiny, stable crate defining the common interface that all zen* codecs implement:
- `Encoding` / `EncodingJob` traits for encode configs
- `Decoding` / `DecodingJob` traits for decode configs
- `PixelData` enum, `ImageInfo`, `ImageMetadata`, `EncodeOutput`, `DecodeOutput`, `DecodeFrame`
- `ImageFormat` enum with magic byte detection
- Re-exports: `imgref`, `rgb`, `enough`

## Design Rules

- `#![no_std]` + `alloc` — must build on wasm32
- `#![forbid(unsafe_code)]`
- Minimal dependencies: `rgb`, `imgref`, `enough`
- No codec-specific types here (those live in the codec crates)
- No `CodecError` here — each codec has its own error type (associated type on trait)
- Traits use GATs for lifetime-parameterized Job types

## Release Requirements

**CI MUST pass before any crates.io release.** This includes:
- All tests pass on Linux, Windows, macOS
- WASM build succeeds (wasm32-wasip1)
- Clippy clean (no warnings)
- Format check passes
- MSRV 1.93 check passes
- `cargo-semver-checks` passes (no unintended breaking changes)

**Before publishing:**
1. Verify README.md reflects current API
2. Run `cargo semver-checks check-release` locally
3. Bump version in Cargo.toml
4. Get explicit user approval
5. `cargo publish`

## Known Issues

(none)
