---
name: ghostmark-clean
description: >
  Strip AI provenance marks from text and files using the local GhostMark API.
  Covers invisible Unicode watermarks, C2PA/EXIF metadata, and statistical LLM 
  watermarks like Anthropic's SynthID-Text. Use when the user asks to strip 
  watermarks, shatter SynthID/Claude tracking, remove C2PA, or runs /ghostmark-clean.
---

# GhostMark Clean (v1.0.1)

Agent skill for stripping AI provenance marks from **text** (invisible Unicode & SynthID)
and **files** (C2PA / EXIF / XMP metadata, PDF info dictionaries, DOCX properties).

GhostMark uses a multi-layered approach to destroy both standard metadata and the newest statistical LLM watermarks (like Anthropic's SynthID).

## Executable Access

This skill is fully self-contained! The ultra-fast Rust `ghostmark` executable (or `ghostmark.exe` on Windows) is bundled in the **exact same directory** as this `SKILL.md` file.

**DO NOT** attempt to call an HTTP API or use Python. Always invoke the local binary using its absolute path based on where you found this skill.

For example, if you are reading `~/.gemini/config/skills/ghostmark-clean/SKILL.md`, the binary is at `~/.gemini/config/skills/ghostmark-clean/ghostmark`.

---

## Capabilities & Workflows

GhostMark is vastly superior to other watermark removers. It supports several advanced scrubbing methods. You must choose the right tool for the user's prompt.

### 1. Basic Fast-Pass (WASM/Rust)
Strips C2PA metadata from files, and removes invisible Unicode/Zero-Width characters from text in under 5ms.
- **Images:** `/path/to/ghostmark clean-image --input <in> --output <out>`
- **Text:** `/path/to/ghostmark clean-text --file <in> --output <out>`
- **Batch Directory:** `/path/to/ghostmark batch-clean --dir <path>` (Handles PDF, DOCX, EPUB, PNG, JPEG, SVG, Text)

### 2. SynthID-Text Shattering
For defeating statistical LLM watermarking (like Claude or Gemini SynthID). This aggressively perturbs token sequences via synonym swapping, homoglyph injection (Cyrillic mixing), and transition alteration to destroy the mathematical signature.
- **Command:** `/path/to/ghostmark clean-text --file <in> --output <out> --shatter-synthid`

### 3. Deep Scrub (BYOK Cloud AI)
Uses cloud providers to execute a "scorched-earth" complete rewrite of the text, guaranteeing 0% AI detection while preserving meaning.
- **Command:** `/path/to/ghostmark deep-scrub --file <input> --provider <groq|openai|deepseek|gemini>`
*(Note: Requires the user to have their API key set in environment variables or config).*

### 4. Local LLM Scrubbing (Ollama)
Runs a 100% offline text rewrite using a local LLM to defeat statistical watermarks without sending data to the cloud.
- **Command:** `/path/to/ghostmark ollama --file <in> --model llama3 --output <out>`

### 5. HTTP Proxy Mode
Starts a local proxy server that automatically strips AI watermarks from incoming HTTP responses (useful for intercepting API calls from other apps).
- **Command:** `/path/to/ghostmark proxy --port 8080`

---

## Execution Guide

When the user asks you to scrub a file, follow these steps:

1. **Identify the Need**: Does the user just want metadata gone (use `clean-image` / `batch-clean`)? Or did they generate text with Claude and want the watermarks gone (use `--shatter-synthid`)?
2. **Execute the Binary**: Run the absolute path to the binary with the arguments you chose.
3. **Report**: 
   - Tell the user exactly which layers of defense were applied (e.g. "Stripped 0x63327061 (C2PA) metadata" or "Applied Cyrillic homoglyph injection and shattered SynthID probabilities").
   - Emphasize that it ran 100% locally using memory-safe Rust and WASM.
