use bytes::Bytes;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use std::fs;

/// Safely strips out cryptographic C2PA signatures and other tracking metadata
/// from JPEG and PNG images without modifying the underlying pixel data.
pub fn strip_image_metadata(
    input_path: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_bytes = fs::read(input_path)?;
    let cleaned = strip_image_bytes(&input_bytes)?;
    fs::write(output_path, cleaned)?;
    Ok(())
}

/// Strips metadata from raw image bytes in memory. Used by the HTTP proxy
/// to avoid touching the filesystem for maximum speed and security.
///
/// Supports JPEG, PNG, BMP, and GIF formats.
pub fn strip_image_bytes(raw: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = Bytes::from(raw.to_vec());

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
        return Ok(out);
    }

    // Try parsing as PNG
    if let Ok(mut png) = Png::from_bytes(bytes) {
        let chunks = png.chunks_mut();
        // Drop tracking chunks (c2pa, iTXt, tEXt, eXIf).
        // We strictly allowlist ONLY the critical rendering chunks.
        chunks.retain(|chunk| {
            let kind = chunk.kind();
            kind == *b"IHDR"
                || kind == *b"PLTE"
                || kind == *b"IDAT"
                || kind == *b"IEND"
                || kind == *b"tRNS"
        });

        let mut out = Vec::new();
        png.encoder().write_to(&mut out)?;
        return Ok(out);
    }

    // Try parsing as BMP (magic bytes: "BM")
    if raw.len() >= 14 && raw[0] == b'B' && raw[1] == b'M' {
        return strip_bmp_trailing_bytes(raw);
    }

    // Try parsing as GIF (magic bytes: "GIF87a" or "GIF89a")
    if raw.len() >= 6 && &raw[0..3] == b"GIF" {
        return strip_gif_trailing_bytes(raw);
    }

    Err("Unsupported file format or corrupted image. Supported: JPEG, PNG, BMP, GIF.".into())
}

/// Strips trailing bytes from BMP files.
///
/// BMP files declare their total file size at bytes 2-5 (little-endian u32)
/// in the BITMAPFILEHEADER. Anything appended after that declared size is
/// extraneous data (potentially steganographic or tracking payloads).
/// We simply truncate the file to the declared size.
fn strip_bmp_trailing_bytes(raw: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if raw.len() < 14 {
        return Err("BMP file too small to contain a valid header.".into());
    }

    // Read the declared file size from offset 2 (4 bytes, little-endian)
    let declared_size = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]) as usize;

    if declared_size < 14 || declared_size > raw.len() {
        // If the declared size is invalid or larger than the actual file,
        // return the file as-is to avoid corruption.
        return Ok(raw.to_vec());
    }

    // Truncate to the declared size, stripping any trailing appended data
    Ok(raw[..declared_size].to_vec())
}

/// Strips trailing bytes from GIF files.
///
/// GIF files end with a trailer byte (0x3B). Any data appended after the
/// trailer is extraneous. We parse through the GIF block structure and
/// stop at the trailer, discarding everything after it.
fn strip_gif_trailing_bytes(raw: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if raw.len() < 13 {
        return Err("GIF file too small to contain a valid header.".into());
    }

    // Verify GIF signature
    if &raw[0..6] != b"GIF87a" && &raw[0..6] != b"GIF89a" {
        return Err("Not a valid GIF file.".into());
    }

    let mut pos: usize = 6;

    // ── Logical Screen Descriptor (7 bytes) ─────────────────────────
    if pos + 7 > raw.len() {
        return Err("GIF truncated at logical screen descriptor.".into());
    }
    let packed = raw[pos + 4];
    let has_gct = (packed & 0x80) != 0;
    let gct_size = if has_gct {
        3 * (1 << ((packed & 0x07) as usize + 1))
    } else {
        0
    };
    pos += 7;

    // ── Global Color Table ──────────────────────────────────────────
    pos += gct_size;
    if pos > raw.len() {
        return Err("GIF truncated in global color table.".into());
    }

    // ── Walk blocks ─────────────────────────────────────────────────
    loop {
        if pos >= raw.len() {
            // Reached end without trailer — return everything
            return Ok(raw.to_vec());
        }

        match raw[pos] {
            // Trailer — end of GIF data
            0x3B => {
                pos += 1; // include the trailer byte itself
                break;
            }
            // Image Descriptor
            0x2C => {
                if pos + 10 > raw.len() {
                    return Ok(raw.to_vec());
                }
                let img_packed = raw[pos + 9];
                let has_lct = (img_packed & 0x80) != 0;
                let lct_size = if has_lct {
                    3 * (1 << ((img_packed & 0x07) as usize + 1))
                } else {
                    0
                };
                pos += 10 + lct_size;

                // Skip LZW minimum code size byte
                if pos >= raw.len() {
                    return Ok(raw.to_vec());
                }
                pos += 1;

                // Skip sub-blocks
                pos = skip_sub_blocks(raw, pos)?;
            }
            // Extension block
            0x21 => {
                if pos + 2 > raw.len() {
                    return Ok(raw.to_vec());
                }
                pos += 2; // skip introducer + label

                // Skip sub-blocks
                pos = skip_sub_blocks(raw, pos)?;
            }
            // Unknown block — stop here
            _ => {
                break;
            }
        }
    }

    // Truncate everything after the valid GIF structure
    Ok(raw[..pos].to_vec())
}

/// Skips GIF sub-blocks starting at `pos`. Each sub-block starts with a
/// size byte; a size of 0 terminates the sequence.
fn skip_sub_blocks(raw: &[u8], mut pos: usize) -> Result<usize, Box<dyn std::error::Error>> {
    loop {
        if pos >= raw.len() {
            return Ok(pos);
        }
        let block_size = raw[pos] as usize;
        pos += 1;
        if block_size == 0 {
            break;
        }
        pos += block_size;
    }
    Ok(pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use img_parts::png::Png;

    fn png_signature() -> [u8; 8] {
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    }

    fn png_chunk(kind: [u8; 4], contents: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(contents.len() as u32).to_be_bytes());
        out.extend_from_slice(&kind);
        out.extend_from_slice(contents);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&kind);
        hasher.update(contents);
        out.extend_from_slice(&hasher.finalize().to_be_bytes());
        out
    }

    fn make_png_with_text() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&png_signature());
        // IHDR: 1x1 8-bit RGBA
        let mut ihdr = vec![0u8; 13];
        ihdr[0] = 0;
        ihdr[1] = 0;
        ihdr[2] = 8; // bit depth
        ihdr[3] = 6; // RGBA
        out.extend_from_slice(&png_chunk(*b"IHDR", &ihdr));
        out.extend_from_slice(&png_chunk(*b"tEXt", b"Comment\0AI generated"));
        out.extend_from_slice(&png_chunk(*b"IDAT", &[0x78, 0x9C, 0x00, 0x00]));
        out.extend_from_slice(&png_chunk(*b"IEND", &[]));
        out
    }

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        let len = (payload.len() + 2) as u16;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn make_jpeg_with_tracking_segments() -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8]; // SOI
        out.extend_from_slice(&jpeg_segment(
            0xE0,
            b"JFIF\x00\x01\x02\x00\x00\x01\x00\x00\x01\x00",
        ));
        out.extend_from_slice(&jpeg_segment(0xE1, b"Exif\x00\x00TRACKING"));
        out.extend_from_slice(&jpeg_segment(0xEB, b"C2PA JUMBF data"));
        out.extend_from_slice(&jpeg_segment(0xED, b"Photoshop 3.0"));
        out.extend_from_slice(&[0xFF, 0xD9]); // EOI
        out
    }

    #[test]
    fn test_png_strips_text_chunks_keeps_rendering() {
        let raw = make_png_with_text();
        let cleaned = strip_image_bytes(&raw).expect("strip ok");

        assert!(
            cleaned.starts_with(&png_signature()),
            "output must still be a PNG"
        );
        let png = Png::from_bytes(Bytes::from(cleaned)).expect("reparse output PNG");
        // tEXt / iTXt / eXIf must be gone; IHDR + IDAT + IEND survive.
        assert!(png.chunk_by_type(*b"tEXt").is_none());
        assert!(png.chunk_by_type(*b"iTXt").is_none());
        assert!(png.chunk_by_type(*b"eXIf").is_none());
        assert!(png.chunk_by_type(*b"c2pa").is_none());
        assert!(png.chunk_by_type(*b"IHDR").is_some());
        assert!(png.chunk_by_type(*b"IDAT").is_some());
        assert!(png.chunk_by_type(*b"IEND").is_some());
    }

    #[test]
    fn test_png_unmodified_when_no_metadata() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&png_signature());
        raw.extend_from_slice(&png_chunk(*b"IHDR", &[0; 13]));
        raw.extend_from_slice(&png_chunk(*b"IDAT", &[0x78, 0x9C, 0x00]));
        raw.extend_from_slice(&png_chunk(*b"IEND", &[]));
        let cleaned = strip_image_bytes(&raw).expect("strip ok");
        // No ancillary chunks to drop -> exactly the 3 critical chunks survive.
        let png = Png::from_bytes(Bytes::from(cleaned)).expect("valid output");
        assert_eq!(png.chunks().len(), 3);
    }

    #[test]
    fn test_jpeg_strips_app1_app11_app13_keeps_jfif() {
        let raw = make_jpeg_with_tracking_segments();
        let cleaned = strip_image_bytes(&raw).expect("strip ok");

        assert!(
            cleaned.starts_with(&[0xFF, 0xD8]),
            "output must start at SOI"
        );
        assert!(
            cleaned.windows(4).any(|w| w == b"JFIF"),
            "harmless JFIF APP0 must survive"
        );
        assert!(
            !cleaned.windows(4).any(|w| w == b"Exif"),
            "APP1 Exif must be stripped"
        );
        assert!(
            !cleaned.windows(4).any(|w| w == b"JUMB"),
            "APP11 C2PA/JUMBF must be stripped"
        );
        assert!(
            !cleaned.windows(4).any(|w| w == b"Phot"),
            "APP13 Photoshop metadata must be stripped"
        );
    }

    #[test]
    fn test_image_stripper_rejects_garbage() {
        assert!(strip_image_bytes(b"not an image").is_err());
    }

    #[test]
    fn test_bmp_trailing_bytes_stripped() {
        // Create a minimal valid BMP with trailing junk
        let mut bmp = vec![0u8; 58]; // 14 header + 40 DIB + 4 pixel data
        bmp[0] = b'B';
        bmp[1] = b'M';
        // Declared file size = 58 (little-endian)
        bmp[2] = 58;
        bmp[3] = 0;
        bmp[4] = 0;
        bmp[5] = 0;

        // Append trailing junk
        bmp.extend_from_slice(b"TRACKING_PAYLOAD_HERE");

        let cleaned = strip_bmp_trailing_bytes(&bmp).unwrap();
        assert_eq!(cleaned.len(), 58);
        assert!(!cleaned.ends_with(b"TRACKING_PAYLOAD_HERE"));
    }

    #[test]
    fn test_gif_trailing_bytes_stripped() {
        // Minimal GIF89a: header(6) + LSD(7) + trailer(1) = 14 bytes
        let mut gif = vec![
            b'G', b'I', b'F', b'8', b'9', b'a', // signature
            1, 0, 1, 0, 0, 0, 0,    // LSD: 1x1, no GCT
            0x3B, // trailer
        ];

        // Append trailing junk
        gif.extend_from_slice(b"HIDDEN_DATA_AFTER_TRAILER");

        let cleaned = strip_gif_trailing_bytes(&gif).unwrap();
        assert_eq!(cleaned.len(), 14);
        assert_eq!(cleaned.last(), Some(&0x3B));
    }
}
