# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-08-19
### Added
- **X-Ray Vision Dashboard**: A brand new highly visual "wow" feature. GhostMark now intercepts uploaded files and presents a sleek, animated X-Ray scanner. It uses a blazing-fast JS byte-scanner to accurately detect C2PA, EXIF, and zero-width Unicode tracking signatures in under 5ms *before* passing the file to the WASM core for destruction.

## [0.14.0] - 2026-08-19
- **Markdown Support**: Added full markdown rendering to the chat feed in the Web Playground using `react-markdown` and `remark-gfm`, enabling beautiful formatting, bold text, and code blocks for AI engine outputs.
- **SynthID Image Scanning**: Added an advanced vision integration to the Web Playground. When "SynthID Detect" is enabled and an image is uploaded, GhostMark now queries the `gemini-2.5-flash` vision model to analyze visual artifacts and cryptographic signatures (like SynthID) to determine if the image is AI-generated.

### Fixed
- **Mobile Viewport Bug**: Fixed a notorious mobile browser `100vh` layout bug in the Web Playground (`index.css`) that was causing the top navigation bar and logo to be clipped under the browser's address bar. Switched layout constraints to use `100dvh` (Dynamic Viewport Height).

## [0.13.0] - 2026-08-18
### Added
- **SynthID-Text Destroyer**: Armed GhostMark to completely neutralize Claude's new SynthID-Text statistical watermark. Revived the Statistical Humanizer engine in the Rust core to aggressively perturb tokens via synonym swapping and phrasing changes, destroying the mathematical token sequence SynthID relies on.
- **CLI Flags**: Added `--shatter-synthid` flag to `clean-text` and `batch-clean` commands.
- **WASM Integration**: Exported `shatter_synthid_text` to WASM and updated Web Workers.
- **UI Toggles**: Added "Shatter SynthID Watermark" toggles to the Web Playground and Chrome Extension.

## [0.12.1] - 2026-08-18
### Fixed
- **SynthID Toggle UX**: The SynthID Detect toggle will no longer enable if the Gemini API key is missing (instead, just opening settings), preventing annoying "SynthID detection skipped" spam messages in the chat interface.
- **Website Image Processing Bug**: Fixed a severe bug in the website's WASM worker (`wasm-worker.ts`) where it forgot to import and use the image processing WASM function, causing all image uploads (e.g., `.jpg`, `.png`) to fail with an "Unsupported file type" error despite UI claims.
- **Website File Input**: Added an explicit `accept` attribute to the website's file input (`App.tsx`) to properly display document formats (`.pdf`, `.docx`, `.epub`, `.odt`, `.svg`) alongside images in the OS file picker.
- **Extension File Input**: Updated the extension popup file input and drag-and-drop zone to officially accept `.pdf`, `.docx`, `.epub`, `.odt`, and `.svg` files alongside images, matching the WASM engine's capabilities.
- **CI Build Warnings**: Removed the abandoned Phase 5.2 Statistical Humanizer logic from `text_scrubber.rs` entirely, resolving build warnings and fixing a broken test (`test_humanizer_changes_text` -> `test_homoglyphs_injected`).
- **Mobile Responsiveness**: Fixed toggle bar layout on mobile devices so switches no longer overlap or wrap poorly.
- **WebGPU Mobile Crash**: Added a fallback where the "Deep Scrub" quick toggle now prevents the 3.8B WebGPU model from downloading on mobile devices to prevent out-of-memory crashes, instead gracefully defaulting to the Cloud engine.

## [0.12.0] - 2026-08-18
### Added
- **UI/UX Overhaul**: Completely redesigned the Playground to feature an ultra-clean, high-contrast **Light Theme** inspired by FarmJS and Better-Auth. Replaced the generic dark mode and pill shapes with stark zinc whites, crisp 1px borders, and professional typography.
- **Playground UI Parity**: Added quick-action toggle switches to the Web Playground, mirroring the Chrome Extension. Users can now easily switch between "WASM Fast", "Deep Scrub", and "SynthID Detect" modes directly above the input area.
- **Multi-Provider BYOK Engine**: Expanded the "BYOK (Groq)" engine in the Playground to support OpenAI (ChatGPT), Google Gemini, DeepSeek, and Groq APIs. API keys are now securely persisted to the browser's `localStorage`.

### Fixed
- **Homoglyph Injection Bug**: Fixed a severe bug in `ghostmark-core` where the WASM text scrubber was running an abandoned, destructive synonym-replacement algorithm ("Humanizer") instead of injecting zero-width homoglyphs. Homoglyph injection now works flawlessly.

## [0.11.0] - 2026-08-18
### Added
- **WASM Web Worker Multithreading**: Migrated the WASM processing engine in both the Web Playground and Chrome Extension to run inside a dedicated Web Worker. This ensures the UI remains fully responsive with zero-blocking when scrubbing massive files (e.g., 500-page EPUBs or heavy PDFs) using zero-copy `ArrayBuffer` transfers.
- **Gemini SynthID Detection**: Added native integration with the Google Gemini API (`taskType: DETECT_TEXT_WATERMARK`) to both the Playground and Extension. Users can now input their own API key to mathematically score and verify if a SynthID watermark is present in their text before and after scrubbing.

### Changed
- **WebGPU Engine**: Upgraded the in-browser WebGPU text rewriting model from `Llama-3.2-1B-Instruct` (1.2B) to Microsoft's `Phi-3-mini-4k-instruct` (3.8B) for significantly smarter offline paraphrasing on desktop devices.

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
