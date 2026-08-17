# Contributing to GhostMark

Thanks for your interest in contributing to GhostMark! This document provides guidelines for contributing.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/GhostMark.git
   cd GhostMark
   ```
3. **Install Rust** (if you haven't already):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
4. **Build and test:**
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

## Development Workflow

1. Create a new branch for your feature:
   ```bash
   git checkout -b feat/your-feature-name
   ```
2. Make your changes
3. Ensure all tests pass:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --all -- --check
   ```
4. Commit with a descriptive message:
   ```bash
   git commit -m "feat: add support for XYZ format"
   ```
5. Push and open a Pull Request

## Project Structure

```
ghostmark/
├── core/           # Core Rust library (all stripping logic)
│   └── src/
│       ├── text_scrubber.rs       # Unicode watermark removal
│       ├── image_stripper.rs      # JPEG/PNG/BMP/GIF metadata stripping
│       └── document_stripper.rs   # PDF/DOCX/SVG/EPUB/ODT stripping
├── cli/            # CLI binary (clap-based)
│   └── src/
│       ├── main.rs                # CLI subcommands
│       └── proxy.rs               # Axum HTTP server
├── wasm/           # WebAssembly bindings
├── extension/      # Chrome Extension (Manifest V3)
├── skills/         # AI agent skills (Cursor, Windsurf, etc.)
└── docs/           # Documentation
```

## Code Guidelines

- **Error handling:** Use `Result<T, String>` in `core/` for simplicity
- **New modules:** Must be exported from `core/src/lib.rs`
- **CLI commands:** Use `clap` derive macros
- **Binary operations:** Accept `&[u8]`, return `Vec<u8>` — keep functions pure
- **No new crate deps** without discussion — we optimize for small binary/WASM size

## Adding a New File Format

1. Add the stripping function to `core/src/document_stripper.rs` or `core/src/image_stripper.rs`
2. Add the file extension to `batch-clean` in `cli/src/main.rs`
3. Add a unit test
4. Update `skills/ghostmark-clean/references/supported-formats.md`
5. Update `CHANGELOG.md`

## Commit Convention

We use conventional commits:
- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation only
- `refactor:` — Code refactoring
- `test:` — Adding tests
- `chore:` — Build/tooling changes

## Reporting Issues

Use the GitHub issue templates for:
- **Bug reports** — Include steps to reproduce, expected vs actual behavior
- **Feature requests** — Describe the use case and proposed solution

## License

By contributing, you agree that your contributions will be licensed under the project's MIT License.
