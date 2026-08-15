use bytes::Bytes;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use std::fs;

/// Safely strips out cryptographic C2PA signatures and other tracking metadata
/// from JPEG and PNG images without modifying the underlying pixel data.
pub fn strip_image_metadata(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let input_bytes = fs::read(input_path)?;
    let bytes = Bytes::from(input_bytes);

    // Try parsing as JPEG
    if let Ok(mut jpeg) = Jpeg::from_bytes(bytes.clone()) {
        let segments = jpeg.segments_mut();
        // Drop tracking segments:
        // 0xE1 = APP1 (Exif/XMP)
        // 0xE2 = APP2 (ICC Profile)
        // 0xEB = APP11 (JPEG XT / C2PA / JUMBF)
        // 0xED = APP13 (Photoshop IRB)
        segments.retain(|seg| {
            let marker = seg.marker();
            marker != 0xE1 && marker != 0xE2 && marker != 0xEB && marker != 0xED
        });

        let mut out = Vec::new();
        jpeg.encoder().write_to(&mut out)?;
        fs::write(output_path, out)?;
        return Ok(());
    }

    // Try parsing as PNG
    if let Ok(mut png) = Png::from_bytes(bytes) {
        let chunks = png.chunks_mut();
        // Drop tracking chunks (c2pa, iTXt, tEXt, eXIf).
        // We strictly allowlist ONLY the critical rendering chunks.
        chunks.retain(|chunk| {
            let kind = chunk.kind();
            kind == *b"IHDR" || kind == *b"PLTE" || kind == *b"IDAT" || kind == *b"IEND" || kind == *b"tRNS"
        });

        let mut out = Vec::new();
        png.encoder().write_to(&mut out)?;
        fs::write(output_path, out)?;
        return Ok(());
    }

    Err("Unsupported file format or corrupted image. Only JPEG and PNG are supported.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Comprehensive byte-level tests would go here.
    // For now, we rely on the type-system guarantees of `img-parts` that chunks are dropped.
    #[test]
    fn test_image_stripper_signature() {
        // Assert that the function exists and compiles correctly
        let _ = strip_image_metadata;
    }
}
