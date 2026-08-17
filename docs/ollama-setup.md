# GhostMark Ollama Setup Guide

GhostMark has an optional **Ollama Mode** that offloads heavy AI rewriting (Deep Scrub & Grammar Only) to your local GPU instead of running slowly inside the Chrome browser. This allows you to use massive 10 Billion+ parameter models and get near-instant results.

## 1. Install Ollama
Download and install Ollama for your operating system from [ollama.com](https://ollama.com).

## 2. Download a Model
Open your terminal (Command Prompt, PowerShell, or macOS/Linux Terminal) and run:
```bash
ollama run llama3
```
This will download the LLaMA 3 model (an 8B parameter model by Meta) and start the local server. You only need to download it once.

*Note: You can use other models like `mistral`, `gemma`, or `phi3`. Just run `ollama pull <model-name>` first.*

## 3. Enable Ollama in GhostMark
1. Click the GhostMark extension icon.
2. Check the **Ollama** toggle above the input bar.
3. A settings box will appear where you can enter the model name. Default is `llama3`.
4. Paste your text, check "Deep Scrub" or "Grammar Only", and hit the Scrub button.

## How it works
When the **Ollama** toggle is active, GhostMark bypasses its internal WebAssembly engine and sends your text directly to `http://localhost:11434/api/generate`. The text is then processed by your actual GPU for massive performance gains, while retaining 100% privacy since it never leaves your machine.
