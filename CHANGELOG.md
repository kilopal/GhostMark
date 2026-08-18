# Changelog

All notable changes to this project will be documented in this file.

## [0.10.0] - 2026-08-18
### Added
- **Sidebar Chat History**: Added a ChatGPT-style sidebar to the web playground for multi-chat session management. Sessions are persisted instantly to local storage.
- **Session State Management**: Migrated the playground from a single message array to a robust multi-session array with title generation.

## [0.9.0] - 2026-08-18
### Added
- **Docker Support**: Added multi-stage `Dockerfile` and `docker-compose.yml` for instant self-hosted deployments. The final image is under 80 MB thanks to a `debian:bookworm-slim` runtime stage.
- **PDF Metadata Stripping**: New `document_stripper::strip_pdf_metadata()` in `ghostmark-core` using `lopdf`. Strips the `/Info` dictionary and all XMP `/Metadata` streams from PDF files.
- **DOCX Metadata Stripping**: New `document_stripper::strip_docx_metadata()` in `ghostmark-core` using the `zip` crate. Removes `docProps/` and `customXml/` entries from DOCX archives.
- **CLI Batch Support**: The `batch-clean` subcommand now automatically detects and processes `.pdf` and `.docx` files alongside text and images.
- **Docker Docs**: Added `docs/docker.md` with deployment guide, configuration, and API examples.
- **Agent Skills**: Added `skills/ghostmark-clean/` with `SKILL.md` and reference docs for Cursor, Windsurf, Cline, and other AI IDE integrations. Agents auto-discover and learn the GhostMark API.
- **Cursor Integration**: Added `.cursorrules` and `.cursor/rules` for native Cursor IDE project context.
- **Demo Playground Website**: Added a static web playground (`playground/`) for instant browser-based WASM watermarking removal testing.
- **Agent Skills Docs**: Added `docs/agent-skills.md` with setup guide and supported IDEs.
- **BMP Support**: New `strip_bmp_trailing_bytes()` in `image_stripper` — truncates BMP files to their declared file size, stripping any steganographic or tracking payloads appended after the header-declared boundary.
- **GIF Support**: New `strip_gif_trailing_bytes()` in `image_stripper` — parses the GIF block structure and truncates at the `0x3B` trailer byte, removing any data appended after the valid GIF stream.
- **CLI Batch Support**: The `batch-clean` subcommand now processes `.bmp`, `.gif`, `.svg`, `.epub`, and `.odt` files alongside JPEG, PNG, WebP, PDF, and DOCX.
- **SVG Metadata Stripping**: New `strip_svg_metadata()` — removes `<metadata>` blocks, HTML comments, and `data-c2pa-*` attributes from SVG files.
- **EPUB Metadata Stripping**: New `strip_epub_metadata()` — removes `META-INF/signatures`, `META-INF/encryption`, and strips `<metadata>` from OPF package files.
- **ODT Metadata Stripping**: New `strip_odt_metadata()` — removes `meta.xml` and digital signature files from OpenDocument Text archives.
- **OpenAPI Spec**: The HTTP proxy now serves a machine-readable OpenAPI 3.0.3 specification at `GET /openapi.json`.
- **GitHub Actions CI/CD**: Added `.github/workflows/ci.yml` with cross-platform testing (Linux/macOS/Windows), clippy lints, formatting checks, Docker build validation, release binary uploads, and GHCR Docker image publishing on tags.
- **CONTRIBUTING.md**: Added contributor guide with project structure, code guidelines, and development workflow.
- **Issue Templates**: Added GitHub issue templates for bug reports and feature requests.

## [0.8.0] - 2026-08-17
### Added
- **Ollama CLI Support**: Added `ollama` subcommand to the CLI to seamlessly rewrite local text using an Ollama server before injecting homoglyphs.
- **Batch Processing**: Added `batch-clean` subcommand to the CLI to recursively scrub all text, json, md, and image files in a directory in-place.
- **Core Homoglyphs**: Ported the Cyrillic homoglyph perturbation logic from the JS extension into the core Rust library (`ghostmark_core::text_scrubber::apply_homoglyphs`).

### Changed
- **WASM Optimization**: Switched to `wee_alloc` and added a `[profile.release]` targeting WASM size optimization (`opt-level = "z"`, `codegen-units = 1`), massively reducing memory footprint in the browser extension.

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
