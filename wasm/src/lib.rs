use wasm_bindgen::prelude::*;
use ghostmark_core::text_scrubber;
use ghostmark_core::image_stripper;

#[wasm_bindgen]
pub fn sanitize_text_wasm(input: &str) -> String {
    text_scrubber::sanitize_text(input)
}

#[wasm_bindgen]
pub fn strip_image_bytes_wasm(raw: &[u8]) -> Result<Vec<u8>, JsValue> {
    image_stripper::strip_image_bytes(raw).map_err(|e| JsValue::from_str(&e.to_string()))
}
