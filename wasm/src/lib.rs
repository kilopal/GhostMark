use wasm_bindgen::prelude::*;
use ghostmark_core::text_scrubber;
use ghostmark_core::image_stripper;

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
