use ghostmark_core::document_stripper;
use ghostmark_core::eval;
use ghostmark_core::image_stripper;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn sanitize_text_wasm(input: &str, aggressive: bool) -> String {
    ghostmark_core::text_scrubber::sanitize_text(input, aggressive)
}

#[wasm_bindgen]
pub fn shatter_synthid_wasm(input: &str) -> String {
    ghostmark_core::text_scrubber::shatter_synthid_text(input)
}

#[wasm_bindgen]
pub fn strip_image_bytes_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    image_stripper::strip_image_bytes(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn strip_pdf_metadata_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    document_stripper::strip_pdf_metadata(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn strip_docx_metadata_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    document_stripper::strip_docx_metadata(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn strip_svg_metadata_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    document_stripper::strip_svg_metadata(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn strip_epub_metadata_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    document_stripper::strip_epub_metadata(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn strip_odt_metadata_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    document_stripper::strip_odt_metadata(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ─── Watermark Embed/Detect (SynthID-compatible) ────────────────────────────

/// Embed a SynthID-style watermark into text.
/// Returns the watermarked text.
#[wasm_bindgen]
pub fn embed_watermark_wasm(text: &str, key: u64) -> String {
    eval::embed_watermark(text, key)
}

/// Embed a watermark with custom configuration.
/// green_pct: percentage of vocabulary that is "green" (default 50)
/// bias_pct: percentage chance to swap red→green (default 100)
#[wasm_bindgen]
pub fn embed_watermark_configured_wasm(text: &str, key: u64, green_pct: u32, bias_pct: u32) -> String {
    let config = eval::WatermarkConfig {
        green_partition_pct: green_pct,
        embed_bias_pct: bias_pct,
        ..Default::default()
    };
    eval::embed_watermark_with_config(text, key, &config)
}

/// Score text for watermark presence. Returns a JSON string with:
/// { "tokens": N, "green": N, "green_frac": F, "z": F, "is_hit": bool }
#[wasm_bindgen]
pub fn score_watermark_wasm(text: &str, key: u64) -> String {
    let stats = eval::score(text, key);
    let result = serde_json::json!({
        "tokens": stats.tokens,
        "green": stats.green,
        "green_frac": stats.green_frac,
        "z": stats.z,
        "is_hit": stats.is_hit(),
    });
    result.to_string()
}

/// Score with custom configuration.
#[wasm_bindgen]
pub fn score_watermark_configured_wasm(text: &str, key: u64, green_pct: u32) -> String {
    let config = eval::WatermarkConfig {
        green_partition_pct: green_pct,
        ..Default::default()
    };
    let stats = eval::score_with_config(text, key, &config);
    let result = serde_json::json!({
        "tokens": stats.tokens,
        "green": stats.green,
        "green_frac": stats.green_frac,
        "z": stats.z,
        "is_hit": stats.is_hit_with(&config),
    });
    result.to_string()
}

/// Run the full eval pipeline (embed → scrub → measure).
/// Returns a JSON string with before/after z-scores and fidelity.
#[wasm_bindgen]
pub fn run_eval_wasm(text: &str, key: u64) -> String {
    let report = eval::run_eval(text, key);
    let result = serde_json::json!({
        "key": report.key,
        "tokens": report.tokens,
        "z_before": report.z_before,
        "z_fast": report.z_fast,
        "z_homoglyph": report.z_homoglyph,
        "z_shatter": report.z_shatter,
        "green_before": report.green_before,
        "green_shatter": report.green_shatter,
        "fidelity_shatter": report.fidelity_shatter,
        "entropy_before": report.entropy_before,
        "entropy_shatter": report.entropy_shatter,
    });
    result.to_string()
}
