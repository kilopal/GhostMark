use ghostmark_core::document_stripper;
use ghostmark_core::image_stripper;
use ghostmark_core::text_scrubber;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn sanitize_text_wasm(input: &str, aggressive: bool) -> String {
    ghostmark_core::text_scrubber::sanitize_text(input, aggressive)
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
