# Changelog

All notable changes to this project will be documented in this file.

## [0.7.0] - 2026-08-17
### Added
- **Ollama Mode**: Introduced an optional toggle to offload heavy AI rewriting to a local Ollama server (`http://localhost:11434`), allowing users to run massive 10B+ parameter models natively on their GPU for instant processing speeds.

### Fixed
- **UI**: Fixed the GhostMark SVG logo in the extension popup (restored the missing eyes in the avatar icons).

## [0.6.0] - 2026-08-17
### Changed
- **Extension UI**: Redesigned the popup with a card-based feed layout, structured result cards, a refined color system, and intentional micro-animations following Calm UI principles.
- **LLM Engine**: Replaced `@mlc-ai/web-llm` (WebGPU-only) with `@huggingface/transformers` (ONNX WASM) and upgraded from flan-t5 to **Llama-3.2-1B-Instruct** (1.2 billion parameters). The Deep Statistical Scrub now runs on **CPU** — no WebGPU required, no deadlocks, works on every device, albeit slower for this huge model size.
- **Bundle Size**: Reduced popup.bundle.js from 6MB to 860KB (7x smaller).
- **WASM Init**: Fixed deprecated parameter warning by passing `{ module_or_path }` object to the wasm-bindgen init function.

## [0.5.0] - 2026-08-17
### Changed
- **Extension UI**: Completely overhauled the browser extension with a state-of-the-art, premium dark mode design. Improved typography, subtle micro-animations, and overall sleek aesthetics.

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
