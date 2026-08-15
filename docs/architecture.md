# GhostMark Architecture

## The Goal
GhostMark aims to provide a reliable, memory-safe utility for completely stripping AI-generated provenance data (such as those mandated by the EU AI Act).

## Core Modules (Planned)

### 1. `text_scrubber`
Focuses on removing zero-width characters (e.g. `U+200B` to `U+200D`) and other invisible unicode markers injected by models like Claude to track output.

### 2. `image_c2pa_stripper`
Safely parses image headers (PNG chunks, JPEG EXIF data) to remove cryptographic C2PA metadata without destroying the image bytes or risking buffer overflows.

### 3. `proxy_middleware` (Future)
Allow GhostMark to be used as a high-speed reverse proxy to sanitize HTTP traffic containing AI output on the fly.
