# SynthID-Text Destroyer

GhostMark includes a specialized "SynthID-Text Destroyer" designed to neutralize cryptographic and statistical token watermarks (like Anthropic's implementation of Google DeepMind's SynthID-Text).

## How SynthID Works
SynthID-Text works by subtly altering the pseudo-random token selection during text generation using a cryptographic key. This leaves a statistical, mathematical signature in the sequence of words chosen by the LLM (e.g., Claude or Gemini). 

## How GhostMark Defeats It
Because the watermark relies entirely on the *exact sequence* of chosen tokens and their probabilities, GhostMark shatters the watermark by heavily perturbing the tokens. 

When the **Shatter SynthID** mode is enabled, GhostMark runs a "Statistical Humanizer" pass that:
1. **Swaps Synonyms**: Automatically replaces AI's favorite words (e.g., *dynamic* → *lively*, *multifaceted* → *layered*) to break the expected token probabilities.
2. **Shifts Transitions**: Alters sentence flow and conjunctions (e.g., *In today's fast-paced world* → *In a fast-moving world*) which shatters the context-window sequences SynthID uses to hide its signature.
3. **Injects Homoglyphs**: After perturbing the words, Cyrillic homoglyphs are injected, completely destroying the BPE tokenization that the SynthID detector relies on.

## Usage

### In the Web Playground
1. Click the **Engine Settings** button in the sidebar.
2. Check the **Shatter SynthID Watermark** box.
3. Process your text.

### In the Chrome Extension
1. Open the GhostMark popup.
2. Toggle on **Shatter SynthID**.
3. Paste your text and click Scrub.

### In the CLI
Pass the `--shatter-synthid` flag to the text or batch cleaning commands:

```bash
# Scrub a single text string
ghostmark clean-text "Your text here" --shatter-synthid

# Scrub a file
ghostmark clean-text --file draft.md --output clean.md --shatter-synthid

# Aggressively scrub an entire folder
ghostmark batch-clean --dir ./dataset/ --shatter-synthid
```

### Deep Rewrite (Ollama)
For factual text where synonym swapping isn't enough to defeat the watermark, Anthropic states that *"a complete rewrite where every word is replaced will [remove the watermark]"*. 

You can use GhostMark's local Ollama integration for a complete "scorched earth" rewrite:
```bash
ghostmark ollama --file draft.md --model llama3 --output clean.md
```
