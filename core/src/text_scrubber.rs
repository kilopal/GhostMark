/// Strips known invisible Unicode characters used for steganographic watermarking.
pub fn sanitize_text(input: &str) -> String {
    input
        .chars()
        .filter(|&c| {
            // Check against known invisible/watermarking Unicode ranges
            match c {
                // Zero-width spaces and joiners
                '\u{200B}'..='\u{200F}' => false,
                // Zero-width no-break space (BOM)
                '\u{FEFF}' => false,
                // Unicode Tags Block (often used for hidden ASCII data)
                '\u{E0000}'..='\u{E007F}' => false,
                // Valid visible character
                _ => true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text_is_untouched() {
        let clean = "This is a completely normal string.";
        assert_eq!(sanitize_text(clean), clean);
    }

    #[test]
    fn test_strips_zero_width_spaces() {
        // String with invisible \u{200B} inserted between words
        let dirty = "Hello\u{200B}World";
        let expected = "HelloWorld";
        assert_eq!(sanitize_text(dirty), expected);
    }

    #[test]
    fn test_strips_unicode_tags() {
        // String with invisible tag characters
        let dirty = "Secret\u{E0041}\u{E0042}Message";
        let expected = "SecretMessage";
        assert_eq!(sanitize_text(dirty), expected);
    }

    #[test]
    fn test_strips_mixed_watermarks() {
        let dirty = "\u{FEFF}A\u{200C}N\u{200D}T\u{200E}H\u{200F}R\u{E0000}O\u{E007F}P\u{200B}I\u{FEFF}C";
        let expected = "ANTHROPIC";
        assert_eq!(sanitize_text(dirty), expected);
    }
}
