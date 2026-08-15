# GhostMark 👻

*A blazing-fast, memory-safe tool written in Rust to strip Anthropic, OpenAI, and EU-mandated AI watermarks (C2PA & Unicode) from text and images.*

## Why GhostMark?
With the enforcement of the EU AI Act (Article 50), frontier AI models like Claude and ChatGPT are injecting invisible tracking watermarks and C2PA metadata into generated text and images. **GhostMark** is a zero-trust compliance red-teaming tool designed to securely scrub, mutate, and destroy these tracking signatures.

## Features
- **Unicode Hygiene**: Scrubs zero-width characters and imperceptible text markers.
- **C2PA Stripping**: Memory-safe extraction and destruction of AI metadata from PNG/JPEG files.
- **HTTP Proxy**: High-concurrency JSON API powered by `axum` + `tokio` for real-time sanitization of LLM outputs.
- **Zero Dependencies**: Ships as a single static binary. No Python, no Docker, no `exiftool`.

## Build Instructions
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
