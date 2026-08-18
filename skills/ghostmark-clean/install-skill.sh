#!/usr/bin/env bash
set -e

# SYNOPSIS
# Installs the ghostmark-clean agent skill globally.
#
# DESCRIPTION
# This script compiles the GhostMark Rust CLI in release mode, creates the global skill 
# directories for Antigravity (~/.gemini/config/skills) and Cursor (~/.cursor/skills), 
# and copies the compiled binary and SKILL.md into them.

# Resolve paths
SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SKILL_DIR/../.." && pwd)"

echo -e "\033[1;36m========================================\033[0m"
echo -e "\033[1;36m👻 Installing ghostmark-clean Skill\033[0m"
echo -e "\033[1;36m========================================\033[0m"

# 1. Compile the Rust CLI
echo -e "\n\033[1;33m[1/3] Compiling GhostMark CLI (Release Mode)...\033[0m"
cd "$REPO_ROOT"
if ! cargo build --release -p ghostmark; then
    echo -e "\033[1;31m❌ Compilation failed. Ensure Rust and Cargo are installed.\033[0m"
    exit 1
fi

BINARY_PATH="$REPO_ROOT/target/release/ghostmark"
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "\033[1;31m❌ Expected binary not found at $BINARY_PATH\033[0m"
    exit 1
fi

# 2. Define global skill paths
GEMINI_SKILL_PATH="$HOME/.gemini/config/skills/ghostmark-clean"
CURSOR_SKILL_PATH="$HOME/.cursor/skills/ghostmark-clean"

# 3. Copy files
echo -e "\n\033[1;33m[2/3] Installing to Antigravity (~/.gemini)...\033[0m"
mkdir -p "$GEMINI_SKILL_PATH"
cp -f "$BINARY_PATH" "$GEMINI_SKILL_PATH/ghostmark"
cp -f "$SKILL_DIR/SKILL.md" "$GEMINI_SKILL_PATH/SKILL.md"
echo -e "\033[1;32m✅ Installed at $GEMINI_SKILL_PATH\033[0m"

echo -e "\n\033[1;33m[3/3] Installing to Cursor (~/.cursor)...\033[0m"
mkdir -p "$CURSOR_SKILL_PATH"
cp -f "$BINARY_PATH" "$CURSOR_SKILL_PATH/ghostmark"
cp -f "$SKILL_DIR/SKILL.md" "$CURSOR_SKILL_PATH/SKILL.md"
echo -e "\033[1;32m✅ Installed at $CURSOR_SKILL_PATH\033[0m"

echo -e "\n\033[1;36m🎉 Installation complete! Agents can now use GhostMark locally.\033[0m\n"
