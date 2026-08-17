# Supported Formats

## Text Formats

| Format | Method | What Gets Stripped |
|--------|--------|--------------------|
| `.txt` | API or CLI | Zero-width chars, Unicode tags, invisible formatting |
| `.md` | API or CLI | Same as `.txt` |
| `.json` | API or CLI | Same as `.txt` |
| Any pasted text | API | Same as `.txt` |

## Image Formats

| Format | Method | What Gets Stripped |
|--------|--------|--------------------|
| `.jpg` / `.jpeg` | API or CLI | EXIF, C2PA manifests, XMP, ICC profiles, IPTC |
| `.png` | API or CLI | C2PA manifests, tEXt/iTXt/zTXt chunks, XMP |
| `.webp` | API or CLI | EXIF, XMP, C2PA manifests |
| `.bmp` | API or CLI | Trailing bytes appended after declared file size |
| `.gif` | API or CLI | Trailing bytes appended after GIF trailer (0x3B) |

## Document Formats

| Format | Method | What Gets Stripped |
|--------|--------|--------------------|
| `.pdf` | CLI only | `/Info` dictionary (Author, Creator, Producer), XMP `/Metadata` streams |
| `.docx` | CLI only | `docProps/` (core.xml, app.xml), `customXml/` (AI provenance data) |

## Limitations

- **Audio/Video:** Not supported.
- **SVG:** Not yet supported (planned).
- **EPUB/ODT:** Not yet supported (planned).
- **PDF pixel watermarks:** Only metadata is stripped. Pixel-level watermarks in PDF images are not modified.
- **DOCX content watermarks:** Only metadata properties are stripped. In-document Word watermarks (text overlays) are not modified.
