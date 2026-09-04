# GhostMark Architecture

GhostMark is built as a highly modular, memory-safe **Cargo Workspace** containing three distinct crates, plus a browser extension.

## 1. `ghostmark-core`
The raw, foundational math engine. 
- **Dependencies**: Only lightweight parsing crates (`img-parts`, `bytes`).
- **Network**: Zero external I/O.
- **Role**: Contains `text_scrubber` (for Unicode/Zero-width metadata), `eval` (for watermark embedding/detection/benchmarking using the Kirchenbauer green/red-list statistical family), and `image_stripper` (for C2PA/Exif manipulation). It operates entirely on raw strings and byte arrays to ensure maximum safety, speed, and cross-platform compatibility (including WASM).

## 2. `ghostmark` (CLI / HTTP Proxy)
The primary backend and terminal application.
- **Dependencies**: Imports `ghostmark-core`, alongside heavy networking crates (`axum`, `tokio`, `clap`).
- **Role**: Exposes a high-performance HTTP server (`/clean/text`, `/clean/image`) for real-time sanitization of LLM API traffic, and handles file-system I/O for the terminal CLI (`ghostmark clean-text`).

## 3. `ghostmark-wasm`
The WebAssembly bindings.
- **Dependencies**: Imports `ghostmark-core`, alongside `wasm-bindgen`.
- **Role**: Compiles the Rust engine into a `.wasm` binary, exposing `sanitize_text_wasm` and `strip_image_bytes_wasm` to Javascript for completely local, offline execution inside the browser.

## 4. `extension/`
A Manifest V3 browser extension for Chrome/Edge/Brave.
- **Design**: Card-based feed layout with structured result cards, built with vanilla HTML/CSS. Uses a refined dark color system and intentional micro-animations.
- **AI Engine**: Uses `@huggingface/transformers` with `onnx-community/Llama-3.2-1B-Instruct` via ONNX WASM for CPU-based text rewriting. No WebGPU required — runs on any device. The model (~800MB - 1.5GB) is downloaded once and cached by the browser.
- **Role**: Connects directly to the `ghostmark-wasm` package (`extension/pkg/`). Intercepts user inputs, passes them to the local WASM engine, and copies the mathematically scrubbed payload directly to the user's clipboard in milliseconds without ever communicating with an external API.

## 5. `playground/`
An interactive web playground for trying the WASM engine natively in the browser.
- **Design**: Mirrors the extension UI with quick-action toggles (WASM Fast, Deep Scrub, SynthID Detect) and multi-chat session management.
- **Role**: Allows instant testing of homoglyph injection and metadata stripping without installing the extension. Integrates with cloud LLMs (OpenAI, Gemini, DeepSeek, Groq) using a BYOK (Bring Your Own Key) approach.
