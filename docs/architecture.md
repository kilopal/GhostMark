# GhostMark Architecture

GhostMark is built as a highly modular, memory-safe **Cargo Workspace** containing three distinct crates, plus a browser extension.

## 1. `ghostmark-core`
The raw, foundational math engine. 
- **Dependencies**: Only lightweight parsing crates (`img-parts`, `bytes`).
- **Network**: Zero external I/O.
- **Role**: Contains `text_scrubber` (for Unicode/Zero-width metadata) and `image_stripper` (for C2PA/Exif manipulation). It operates entirely on raw strings and byte arrays to ensure maximum safety, speed, and cross-platform compatibility (including WASM).

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
- **Design**: Built with a sleek, minimalist, ChatGPT-style chat interface using vanilla HTML/CSS.
- **Role**: Connects directly to the `ghostmark-wasm` package (`extension/pkg/`). Intercepts user inputs, passes them to the local WASM engine, and copies the mathematically scrubbed payload directly to the user's clipboard in milliseconds without ever communicating with an external API.
