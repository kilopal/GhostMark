# API Reference

## Base URL

Default: `http://127.0.0.1:8080`

Override with the `GHOSTMARK_URL` environment variable.

---

## `GET /health`

Returns the service health status.

**Response:**
```json
{
  "ok": true,
  "version": "0.9.0",
  "engine": "GhostMark/Rust"
}
```

---

## `POST /clean/text`

Strips invisible Unicode watermarks from the provided text.

**Request:**
```json
{
  "text": "Hello\u200Bworld"
}
```

**Response:**
```json
{
  "ok": true,
  "original_len": 12,
  "cleaned_len": 10,
  "chars_removed": 2,
  "cleaned": "Helloworld",
  "elapsed_us": 45
}
```

**What it removes:**
- Zero-width spaces (`U+200B`)
- Zero-width non-joiners (`U+200C`)
- Zero-width joiners (`U+200D`)
- Left-to-right / right-to-left marks (`U+200E`, `U+200F`)
- Byte order marks (`U+FEFF`)
- Unicode tag characters (`U+E0000`–`U+E007F`)
- Variation selectors
- Soft hyphens
- Other invisible formatting characters

---

## `POST /inspect/text`

Detects suspicious watermark characters without modifying the text.

**Request:**
```json
{
  "text": "Hello\u200Bworld"
}
```

**Response:**
```json
{
  "ok": true,
  "suspicious": true,
  "suspicious_chars": [
    {
      "position": 5,
      "codepoint": "U+200B",
      "name": "Zero Width Space"
    }
  ],
  "total_suspicious": 1
}
```

---

## `POST /clean/image`

Strips C2PA, EXIF, and XMP metadata from images. Accepts base64-encoded image data.

**Request:**
```json
{
  "file": "<base64-encoded image bytes>",
  "name": "photo.jpg"
}
```

**Response:**
```json
{
  "ok": true,
  "original_size": 245760,
  "cleaned_size": 201540,
  "bytes_removed": 44220,
  "cleaned": "<base64-encoded cleaned image>",
  "elapsed_us": 1230
}
```

**Supported formats:** JPEG, PNG, WebP

---

## Error Responses

All endpoints return errors in a consistent format:

```json
{
  "ok": false,
  "error": "Description of what went wrong"
}
```

HTTP status codes:
- `400` — Bad request (e.g., invalid base64)
- `422` — Unprocessable entity (e.g., unsupported image format)
