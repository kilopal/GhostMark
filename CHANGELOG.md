# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-08-15
### Added
- Implemented `image_c2pa_stripper` module.
- Added `img-parts` dependency for memory-safe JPEG and PNG chunk manipulation.
- Added `clean-image` CLI command to aggressively strip C2PA, EXIF, and non-essential tracking metadata from images.

## [0.1.0] - 2026-08-15
### Added
- Implemented `text_scrubber` core module to securely strip zero-width characters and invisible Unicode steganography.
- Integrated `clap` for a clean CLI interface supporting string and file inputs.

## [0.0.1] - 2026-08-15
### Added
- Initial project scaffolding for GhostMark.
- Setup core Rust binary structure.
- Added documentation for architecture and project vision.
