# Changelog

All notable changes to zencodec are documented here.

## [0.1.6] - 2026-03-28

### Fixed

- `ImageInfo::PartialEq` now includes the `resolution` field (was silently skipped,
  causing two values with different resolutions to compare as equal).
- 10 broken rustdoc intra-doc links (`codec_details` on dyn trait objects,
  `ImageFormatRegistry::with`, `PixelDescriptor` qualification, `Any::downcast_ref`/`Deref` paths).

### Added

- Missing derives on public types: `PartialEq` on `Metadata`, `Clone`/`PartialEq`/`Eq` on
  `DecodeCapabilities`, `Clone`/`PartialEq` on `EncodeCapabilities`, `PartialEq` on
  `GainMapParseError`.

### Changed

- Bumped `zenpixels` dependency from 0.2.0 to 0.2.1 (gamut matrices, serde support,
  embedded ICC profiles, bug fixes).
- README: added badges, ecosystem cross-links, limitations section, MSRV declaration;
  fixed dead guide links and stale `from_magic()` reference.

## [0.1.5] - 2026-03-26

### Changed

- `DecoderConfig::job(self)` now consumes `self` (was `&self`). Uses GAT + method
  lifetime to avoid forcing `'static` on the config.

### Added

- `DecodeJob::with_extract_gain_map()` — opt in to gain map extraction during decode.
- Default impl for `DynDecodeJob::set_extract_gain_map`.

## [0.1.4] - 2026-03-26

### Changed

- Added `Send` supertrait to `DynEncoder` (required for cross-thread encoder dispatch).

## [0.1.3] - 2026-03-25

### Added

- `GainMapSource` — raw gain map data extracted from container (pre-decode).
  Carries raw encoded bitstream + format + ISO 21496-1 metadata + recursion
  depth counter for safe nested decode. Accessible via
  `zencodec::gainmap::GainMapSource`.
- `DecodedGainMap` — decoded gain map pixels + metadata (post-decode).
  Cross-codec normalized type. Accessible via
  `zencodec::gainmap::DecodedGainMap`.
- Both types are `#[non_exhaustive]` with `new()` constructors.

### Changed

- Documented supplement decode convention: detection is always cheap
  (container metadata), pixel decode is opt-in. `ImageInfo.supplements`
  flags describe what's available, not what's decoded.
- Updated `docs/spec.md` with three-layer decode output model
  (ImageInfo, SourceEncodingDetails, Extensions type-map) and
  supplement access conventions.

## [0.1.2] - 2026-03-25

### Added

- `ImageInfo.is_progressive` field — true for progressive JPEG (SOF2),
  interlaced PNG (Adam7), interlaced GIF. Detectable from headers during
  cheap probe.
- `ImageInfo.with_progressive()` builder method.

## [0.1.1] - 2026-03-24

### Changed

- Drop unnecessary `imgref` feature from zenpixels dependency.
- Add magic byte detection audit example.
