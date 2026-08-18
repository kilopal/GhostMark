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
- `references/supported-formats.md` — Which file types are supported and what gets stripped

## Executable Access

This skill is fully self-contained! The ultra-fast Rust `ghostmark` executable (or `ghostmark.exe` on Windows) is bundled in the **exact same directory** as this `SKILL.md` file.

**DO NOT** attempt to call an HTTP API or use Python. Always invoke the local binary using its absolute path based on where you found this skill.

For example, if you are reading `~/.gemini/config/skills/ghostmark-clean/SKILL.md`, the binary is at `~/.gemini/config/skills/ghostmark-clean/ghostmark`.

## Workflow

### 1. Classify Input

Determine the target file type and use the appropriate GhostMark CLI command:

| Input | Command |
| --- | --- |
| `.txt` / `.md` / `.json` / code | `/path/to/ghostmark clean-text --file <input> --output <output>` |
| `.png` / `.jpg` / `.jpeg` / `.webp` | `/path/to/ghostmark clean-image --input <input> --output <output>` |
| Directory / batch processing | `/path/to/ghostmark batch-clean --dir <path>` |

*(Note: GhostMark's `batch-clean` command automatically handles `.pdf`, `.docx`, `.epub`, `.odt`, `.svg`, images, and text files recursively!)*

### 2. Execution

**Text (Standard Scrubbing):**
```bash
/path/to/ghostmark clean-text --file draft.md --output draft.cleaned.md
```

**Text (SynthID-Text Defeat / Statistical Humanizer):**
If you suspect the text contains statistical watermarks like Claude's SynthID-Text, use the `--shatter-synthid` flag to heavily perturb the tokens (synonym swapping, transition changes, etc.) to destroy the watermark sequence:
```bash
/path/to/ghostmark clean-text --file draft.md --output draft.cleaned.md --shatter-synthid
```

**Text (Deep Rewrite via Ollama):**
For a completely "scorched-earth" approach that defeats all statistical watermarks by rewriting the entire text locally:
```bash
/path/to/ghostmark ollama --file draft.md --model llama3 --output draft.rewritten.md
```

**Images:**
```bash
/path/to/ghostmark clean-image --input photo.jpg --output photo.cleaned.jpg
```

**Batch / Folders (including PDF/DOCX):**
```bash
/path/to/ghostmark batch-clean --dir ./my-files/
```
*(You can also pass `--shatter-synthid` to the `batch-clean` command to aggressively perturb all text/markdown files found in the directory).*

### 3. Report

Always state:
- What files were scrubbed.
- Output the GhostMark CLI's success messages (which mention what was stripped).
- Note that GhostMark runs purely locally using memory-safe Rust.
