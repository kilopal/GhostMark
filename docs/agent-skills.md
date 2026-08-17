# Agent Skills & IDE Integration

GhostMark ships with built-in AI agent skills so that AI coding assistants (Cursor, Windsurf, Cline, etc.) can use GhostMark as a tool.

## What Are Agent Skills?

Agent skills are structured instruction files that tell AI assistants how to use an external tool. When an AI agent detects the skill file in your project, it automatically learns how to call the GhostMark API to clean text and strip metadata.

## Supported IDEs

| IDE | Integration File | Auto-Detected? |
|-----|-----------------|----------------|
| **Cursor** | `.cursorrules` + `.cursor/rules` | ✅ Yes |
| **Windsurf** | `skills/ghostmark-clean/SKILL.md` | ✅ Yes |
| **Cline** | `skills/ghostmark-clean/SKILL.md` | ✅ Yes |
| **Antigravity** | `skills/ghostmark-clean/SKILL.md` | ✅ Yes |
| **Any MCP-compatible agent** | Via the HTTP API directly | Manual setup |

## Quick Setup

1. **Start the GhostMark server:**
   ```bash
   # Option A: Docker
   docker compose up -d

   # Option B: Cargo
   cargo run -p ghostmark -- serve
   ```

2. **Open your project in Cursor or another AI IDE.** The agent will automatically detect the skill files and learn how to use GhostMark.

3. **Ask your AI agent to clean text or images.** Example prompts:
   - "Clean the watermarks from `draft.txt`"
   - "Strip metadata from all images in `./assets/`"
   - "Inspect this text for hidden Unicode characters"

## File Structure

```
ghostmark/
├── .cursorrules              # Cursor IDE quick-reference rules
├── .cursor/
│   └── rules                 # Cursor IDE detailed project context
└── skills/
    └── ghostmark-clean/
        ├── SKILL.md           # Main agent skill (workflow + API)
        └── references/
            ├── api-reference.md       # Full HTTP API docs
            └── supported-formats.md   # File format support matrix
```

## How It Works

When your AI agent reads the `SKILL.md` file, it learns:

1. **How to check if GhostMark is running** — by calling `GET /health`
2. **How to inspect text** — by calling `POST /inspect/text` to detect suspicious characters
3. **How to clean text** — by calling `POST /clean/text` to strip watermarks
4. **How to clean images** — by calling `POST /clean/image` with base64-encoded files
5. **How to batch-clean directories** — by running the CLI `ghostmark batch-clean --dir <path>`

The agent automatically handles encoding, decoding, and writing output files.

## Custom Skill Configuration

If your GhostMark server runs on a non-default URL, set the environment variable:

```bash
export GHOSTMARK_URL=http://your-server:9090
```

The agent skill will respect this variable when making API calls.
