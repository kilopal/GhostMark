# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-08-15
### Added
- **Browser Extension**: A complete Chrome/Edge extension with a premium ChatGPT-style UI.
- **WebAssembly (WASM)**: Compiled the core engine to WASM for 0-latency, offline, in-browser sanitization.
- **Cargo Workspace**: Refactored the entire repository into `core`, `cli`, and `wasm` modules for clean architecture.

## [0.3.0] - 2026-08-15
### Added
- Built `proxy` module: a high-concurrency HTTP server powered by `axum` + `tokio`.
- Added `POST /clean/text` endpoint for stripping Unicode watermarks from JSON payloads.
- Added `POST /inspect/text` endpoint for detecting and reporting suspicious watermark characters with exact codepoints and positions.
- Added `POST /clean/image` endpoint for stripping C2PA/Exif metadata from base64-encoded images.
- Added `GET /health` endpoint for service health checks.
- Added `serve` CLI subcommand (`ghostmark serve --port 8080`).
- Refactored `image_stripper` to expose `strip_image_bytes()` for zero-filesystem in-memory processing.

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
