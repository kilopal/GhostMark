# GhostMark Architecture

## The Goal
GhostMark aims to provide a reliable, memory-safe utility for completely stripping AI-generated provenance data (such as those mandated by the EU AI Act).

## Core Modules (Planned)

### 1. `text_scrubber` (Completed)
Strips zero-width characters (`U+200B` to `U+200F`), the zero-width no-break space (`U+FEFF`), and the Unicode Tags Block (`U+E0000` - `U+E007F`). Designed to run locally with zero network calls for maximum opsec.

### 2. `image_c2pa_stripper` (Completed)
Uses the `img-parts` crate to memory-safely parse PNG and JPEG byte streams. It explicitly allows only critical rendering chunks (like `IHDR` and `IDAT`), completely stripping out `APP1` (Exif), `APP11` (C2PA/JUMBF), `c2pa`, and `iTXt` metadata chunks without modifying the visual pixel data.

### 3. `proxy_middleware` (Completed)
High-concurrency HTTP proxy built on `axum` + `tokio`. Exposes `/clean/text`, `/clean/image`, `/inspect/text`, and `/health` endpoints. Processes all data in-memory (zero filesystem I/O) for maximum speed and security. Can be deployed as a standalone microservice or sidecar container.
