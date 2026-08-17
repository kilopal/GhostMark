---
name: ghostmark-clean
description: >
  Strip AI provenance marks from text and files using the local GhostMark API.
  Covers invisible Unicode watermarks (zero-width chars, tag characters),
  C2PA/EXIF/XMP metadata on PNG/JPEG/WebP images, and document metadata on
  PDF/DOCX files. Use when the user asks to strip watermarks, remove C2PA,
  clean AI metadata, remove invisible Unicode, or runs /ghostmark-clean.
---

# GhostMark Clean

Agent skill for stripping AI provenance marks from **text** (invisible Unicode)
and **files** (C2PA / EXIF / XMP metadata, PDF info dictionaries, DOCX properties).

Read if needed:

- `references/api-reference.md` — Full HTTP API documentation
- `references/supported-formats.md` — Which file types are supported and what gets stripped

## Service Access

Base URL comes from `GHOSTMARK_URL`, default `http://127.0.0.1:8080`:

```bash
GM="${GHOSTMARK_URL:-http://127.0.0.1:8080}"
```

The service is started via Docker (`docker compose up -d`) or locally
(`cargo run -p ghostmark -- serve`). **Always check health first**, and stop
with a clear message if unreachable:

```bash
curl -sf "$GM/health"
# {"ok": true, "version": "...", "engine": "GhostMark/Rust"}
```

## HTTP API

| Method | Path | Body | Returns |
| --- | --- | --- | --- |
| `GET` | `/health` | — | `{"ok", "version", "engine"}` |
| `POST` | `/clean/text` | `{"text": "..."}` | `{"ok", "original_len", "cleaned_len", "chars_removed", "cleaned", "elapsed_us"}` |
| `POST` | `/inspect/text` | `{"text": "..."}` | `{"ok", "suspicious", "suspicious_chars", "total_suspicious"}` |
| `POST` | `/clean/image` | `{"file": "<base64>", "name": "photo.jpg"}` | `{"ok", "original_size", "cleaned_size", "bytes_removed", "cleaned", "elapsed_us"}` |

## Workflow

### 1. Classify Input

| Input | Route |
| --- | --- |
| Pasted / clipboard text | `/inspect/text` then `/clean/text` |
| `.txt` / `.md` / `.json` / code | `/clean/text` (read file, send contents) |
| `.png` / `.jpg` / `.jpeg` / `.webp` | `/clean/image` (base64 encode, send) |
| `.pdf` | CLI: `ghostmark batch-clean --dir .` |
| `.docx` | CLI: `ghostmark batch-clean --dir .` |
| Directory | CLI: `ghostmark batch-clean --dir <path>` |

### 2. Inspect First (Text)

Always inspect before cleaning so you can report what was found:

```bash
curl -s -X POST "$GM/inspect/text" \
  -H "Content-Type: application/json" \
  -d "{\"text\": \"$(cat file.txt | jq -Rs .)\"}"
```

Show a short summary: number of suspicious codepoints, their types (Zero Width
Space, Unicode Tag Character, etc.), and positions.

### 3. Clean

**Text:**

```bash
curl -s -X POST "$GM/clean/text" \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello\u200Bworld"}'
```

Write the `cleaned` field back to the file or display it.

**Images:**

```bash
curl -s -X POST "$GM/clean/image" \
  -H "Content-Type: application/json" \
  -d "{\"file\": \"$(base64 < photo.jpg | tr -d '\n')\", \"name\": \"photo.jpg\"}"
```

Decode the returned `cleaned` base64 into the output file.

On Windows agents, build base64 with:
```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("photo.jpg"))
```

### 4. Report

Always state:
- What was found (count and types of suspicious characters, or metadata presence)
- What was removed (chars_removed for text, bytes_removed for images)
- Processing time (elapsed_us)
- Write output to `*.cleaned.*` unless the user asked for in-place editing

## CLI Fallback

If the HTTP service is not running, you can use the CLI directly:

```bash
# Clean a single text file
ghostmark clean-text --file input.txt --output clean.txt

# Clean an image
ghostmark clean-image --input photo.jpg --output photo.clean.jpg

# Batch clean a directory (text, images, PDF, DOCX)
ghostmark batch-clean --dir ./my-files/
```

Build the CLI with `cargo build --release -p ghostmark`.

## Service Not Reachable?

If `$GM/health` fails, tell the user to start the service:

```bash
# Option 1: Docker
docker compose up -d

# Option 2: Cargo
cargo run -p ghostmark -- serve --host 127.0.0.1 --port 8080
```

Do **not** attempt to clean manually — use the GhostMark engine.
