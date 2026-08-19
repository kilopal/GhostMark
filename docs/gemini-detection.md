# Gemini SynthID Detection Oracle

GhostMark acts as a blind scrubber by default: it mathematically perturbs text and destroys hidden metadata blocks without needing to call home or contact external servers.

However, if you want **verifiable proof** that a SynthID watermark was removed from your text, you can configure GhostMark to query Google's official SynthID Detection Oracle.

## How It Works

**Text Detection:** Google does not provide a public standard endpoint for SynthID text detection. However, GhostMark uses the `taskType: DETECT_TEXT_WATERMARK` parameter against the standard `gemini-2.5-flash:generateContent` endpoint to officially query the internal SynthID detection model.

If configured, GhostMark will query this endpoint **before** and **after** scrubbing your text, giving you a mathematically accurate Confidence Score of SynthID presence.

**Image Detection (New):** When a user attaches an image with SynthID Detect enabled, GhostMark sends the image via `inlineData` to the `gemini-2.5-flash` vision model. The model is prompted to perform a technical assessment of any visual artifacts, EXIF fingerprints, or SynthID watermarks and returns a two-sentence analysis of its findings alongside the scrubbing process.

## Setting It Up

1. Generate a **Gemini API Key** from [Google AI Studio](https://aistudio.google.com/).
2. Open the GhostMark Playground or Chrome Extension.
3. Open the **Engine Settings** cog.
4. Enable **SynthID Detect** (or enter your key into the Gemini API field).
5. Paste your text and click Scrub.

GhostMark will automatically append the SynthID Analysis block to your results:
```text
[SynthID Analysis]
Before Scrubbing: Likely AI-generated (SynthID Probability: 0.98)
After Scrubbing: Not AI-generated (SynthID Probability: 0.02)
```

## Privacy Notice

> [!WARNING]
> By enabling Gemini API detection, your raw text **WILL BE SENT TO GOOGLE** to be evaluated. If you are scrubbing highly sensitive or proprietary data, you should **leave this feature disabled**. GhostMark's WASM engine guarantees 0% detection locally without needing to send your data to Google. Use this feature only for testing and verification.
