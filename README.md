# GhostMark 👻

*A blazing-fast, memory-safe tool written in Rust to strip Anthropic, OpenAI, and EU-mandated AI watermarks (C2PA & Unicode) from text and images.*

## Why GhostMark?
With the enforcement of the EU AI Act (Article 50), frontier AI models like Claude and ChatGPT are injecting invisible tracking watermarks and C2PA metadata into generated text and images. **GhostMark** is a zero-trust compliance red-teaming tool designed to securely scrub, mutate, and destroy these tracking signatures.

## Features
- **Universal**: Strips zero-width characters, invisible tags, and C2PA cryptographic image signatures.
- **Memory-Safe**: Written in 100% Rust. No buffer overflows.
- **High Concurrency**: Built-in HTTP proxy (`axum` + `tokio`) capable of millions of requests per second.
- **Browser Native**: Includes a compiled WebAssembly (WASM) Chrome/Edge Extension for local, offline sanitization.
- **Standalone Binary**: No Python, no Docker, no system dependencies.

## Architecture

GhostMark is structured as a highly modular **Cargo Workspace**:
- `core/`: The raw, memory-safe math algorithms. Zero external network dependencies.
- `cli/`: The terminal application and HTTP proxy server.
- `wasm/`: The browser bindings and WebAssembly module.
- `extension/`: The premium browser extension UI.

## Installation

### Option 1: Browser Extension (Recommended)
1. Open Chrome/Edge and go to `chrome://extensions/`
2. Enable **Developer mode**.
3. Click **Load unpacked** and select the `GhostMark/extension` folder.
4. Click the GhostMark icon in your toolbar, paste text, and hit Enter.

### Option 2: Build the CLI/Server from source
1. Install Rust (`cargo`)
2. Run `cargo build --release`
3. Execute `./target/release/ghostmark clean-text <target>`

## Usage

**Clean text directly from the command line:**
```bash
ghostmark clean-text "Text with hidden \u200B marks"
```

**Clean text from a file:**
```bash
ghostmark clean-text input.txt --file --output clean.txt
```

**Clean tracking metadata (C2PA/Exif) from an image:**
```bash
ghostmark clean-image --input watermarked.png --output safe.png
```

**Start the HTTP proxy server:**
```bash
ghostmark serve --port 8080
```

**Send a request to the proxy:**
```bash
curl -X POST http://127.0.0.1:8080/clean/text \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello\u200bWorld"}'
```
