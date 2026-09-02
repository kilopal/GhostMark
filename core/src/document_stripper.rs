use lopdf::Document;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub fn strip_pdf_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(bytes).map_err(|e| format!("Failed to parse PDF: {}", e))?;

    // Remove the Document Information Dictionary from the trailer
    doc.trailer.remove(b"Info");

    // Remove Metadata streams from the document
    let mut metadata_ids = Vec::new();
    for (object_id, object) in doc.objects.iter() {
        if let lopdf::Object::Stream(stream) = object {
            if let Ok(type_name) = stream.dict.get(b"Type") {
                if let Ok(name) = type_name.as_name() {
                    if name == b"Metadata" {
                        metadata_ids.push(*object_id);
                    }
                }
            }
        }
    }

    for id in metadata_ids {
        doc.objects.remove(&id);
    }

    let mut out_buffer = Vec::new();
    doc.save_to(&mut out_buffer)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(out_buffer)
}

pub fn strip_docx_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to parse DOCX (ZIP): {}", e))?;

    let mut out_buffer = Vec::new();
    let cursor = Cursor::new(&mut out_buffer);
    let mut zip_writer = ZipWriter::new(cursor);

    let options = SimpleFileOptions::default();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Error reading ZIP file: {}", e))?;
        let name = file.name().to_string();

        // Skip docProps/ and customXml/ as they contain author and provenance metadata
        if name.starts_with("docProps/") || name.starts_with("customXml/") {
            continue;
        }

        zip_writer
            .start_file(name, options)
            .map_err(|e| format!("Error writing ZIP file: {}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Error reading file content: {}", e))?;
        zip_writer
            .write_all(&buffer)
            .map_err(|e| format!("Error writing file content: {}", e))?;
    }

    // Convert back from mutable cursor reference to owned Vec<u8>
    let cursor = zip_writer
        .finish()
        .map_err(|e| format!("Failed to finalize DOCX (ZIP): {}", e))?;
    out_buffer = cursor.into_inner().to_vec();

    Ok(out_buffer)
}

/// Strips metadata from SVG files.
///
/// SVG is XML-based, so we remove `<metadata>...</metadata>` blocks,
/// `<dc:...>` Dublin Core elements, and common AI-injected attributes
/// like `data-c2pa-*` without needing a full XML parser.
pub fn strip_svg_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let content =
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("SVG is not valid UTF-8: {}", e))?;

    let mut result = content.clone();

    // Remove <metadata>...</metadata> blocks (case-insensitive, multiline)
    while let Some(start) = find_ci(&result, "<metadata") {
        if let Some(end_rel) = find_ci(&result[start..], "</metadata>") {
            let end_abs = start + end_rel + "</metadata>".len();
            result = format!("{}{}", &result[..start], &result[end_abs..]);
        } else {
            break;
        }
    }

    // Remove <!-- comments --> that may contain provenance info
    while let Some(start) = result.find("<!--") {
        if let Some(end) = result[start..].find("-->") {
            let end_abs = start + end + "-->".len();
            result = format!("{}{}", &result[..start], &result[end_abs..]);
        } else {
            break;
        }
    }

    // Remove data-c2pa-* attributes. Repeat until none remain; a malformed
    // attribute (missing `=` or an unquoted value) must not abort the loop.
    while let Some(start) = find_ci(&result, "data-c2pa-") {
        // Find the value boundary: a `="..."` / `='...'` pair if well-formed,
        // otherwise the bare token ends at the next whitespace / `>` / `/`.
        let mut end_abs = start;
        if let Some(eq_rel) = result[start..].find('=') {
            let after_eq = start + eq_rel + 1;
            if let Some(quote) = result[after_eq..].chars().next() {
                if quote == '"' || quote == '\'' {
                    if let Some(end_rel) = result[after_eq + quote.len_utf8()..].find(quote) {
                        let after_value = after_eq + quote.len_utf8() + end_rel + quote.len_utf8();
                        if after_value <= result.len() {
                            end_abs = after_value;
                        }
                    }
                }
            }
        }
        if end_abs == start {
            let rest = &result[start..];
            let token_end = rest
                .char_indices()
                .skip(1)
                .find(|(_, c)| c.is_whitespace() || *c == '>' || *c == '/')
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            end_abs = start + token_end;
        }
        if end_abs <= start {
            break;
        }

        // Also remove the leading whitespace so the attribute leaves no gap.
        let attr_start = result[..start].rfind(char::is_whitespace).unwrap_or(start);
        result = format!("{}{}", &result[..attr_start], &result[end_abs..]);
    }

    Ok(result.into_bytes())
}

/// Case-insensitive, byte-exact search for an ASCII needle. Returns the byte
/// index of the first match, or `None`. Only meaningful for ASCII needles (all
/// callers pass ASCII tag/attribute names); non-ASCII input is skipped safely.
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let needle_lower = needle.to_ascii_lowercase();
    let nbytes = needle.len();
    haystack.char_indices().find_map(|(i, _)| {
        let end = i + nbytes;
        if end <= haystack.len()
            && haystack.is_char_boundary(end)
            && haystack.as_bytes()[i..end].to_ascii_lowercase() == needle_lower.as_bytes()
        {
            Some(i)
        } else {
            None
        }
    })
}

/// Strips metadata from EPUB files.
///
/// EPUB is a ZIP archive. We remove `META-INF/` directory entries
/// (which contain container.xml, signatures, and encryption info)
/// and strip the `<metadata>` block from the OPF content file.
pub fn strip_epub_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to parse EPUB (ZIP): {}", e))?;

    let mut out_buffer = Vec::new();
    let cursor = Cursor::new(&mut out_buffer);
    let mut zip_writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Error reading EPUB entry: {}", e))?;
        let name = file.name().to_string();

        // Skip digital signatures and encryption metadata
        if name.starts_with("META-INF/signatures")
            || name.starts_with("META-INF/encryption")
            || name.starts_with("META-INF/rights")
        {
            continue;
        }

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Error reading EPUB content: {}", e))?;

        // Strip metadata from OPF files (the package document)
        if name.ends_with(".opf") {
            if let Ok(content) = String::from_utf8(buffer.clone()) {
                let cleaned = strip_xml_metadata_block(&content);
                buffer = cleaned.into_bytes();
            }
        }

        zip_writer
            .start_file(name, options)
            .map_err(|e| format!("Error writing EPUB entry: {}", e))?;
        zip_writer
            .write_all(&buffer)
            .map_err(|e| format!("Error writing EPUB content: {}", e))?;
    }

    let cursor = zip_writer
        .finish()
        .map_err(|e| format!("Failed to finalize EPUB: {}", e))?;
    out_buffer = cursor.into_inner().to_vec();

    Ok(out_buffer)
}

/// Strips metadata from ODT (OpenDocument Text) files.
///
/// ODT is a ZIP archive. We remove `meta.xml` (document metadata)
/// and strip `META-INF/` signature files.
pub fn strip_odt_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to parse ODT (ZIP): {}", e))?;

    let mut out_buffer = Vec::new();
    let cursor = Cursor::new(&mut out_buffer);
    let mut zip_writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Error reading ODT entry: {}", e))?;
        let name = file.name().to_string();

        // Skip meta.xml (contains author, creation date, editing stats)
        if name == "meta.xml" {
            continue;
        }

        // Skip digital signature files
        if name.starts_with("META-INF/documentsignatures") {
            continue;
        }

        zip_writer
            .start_file(name, options)
            .map_err(|e| format!("Error writing ODT entry: {}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Error reading ODT content: {}", e))?;
        zip_writer
            .write_all(&buffer)
            .map_err(|e| format!("Error writing ODT content: {}", e))?;
    }

    let cursor = zip_writer
        .finish()
        .map_err(|e| format!("Failed to finalize ODT: {}", e))?;
    out_buffer = cursor.into_inner().to_vec();

    Ok(out_buffer)
}

/// Helper: strips `<metadata>...</metadata>` and `<dc:*>` blocks from XML content.
fn strip_xml_metadata_block(content: &str) -> String {
    let mut result = content.to_string();

    // Remove <metadata>...</metadata> blocks
    while let Some(start) = result.find("<metadata") {
        if let Some(end) = result[start..].find("</metadata>") {
            let end_abs = start + end + "</metadata>".len();
            result = format!("{}{}", &result[..start], &result[end_abs..]);
        } else {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};

    fn read_zip_names(bytes: &[u8]) -> Vec<String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut out_buffer = Vec::new();
        let cursor = Cursor::new(&mut out_buffer);
        let mut zw = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        for (name, content) in entries {
            zw.start_file(*name, options).unwrap();
            zw.write_all(content.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
        out_buffer
    }

    #[test]
    fn test_pdf_strips_trailer_info_and_metadata_stream() {
        let mut doc = Document::with_version("1.7");
        let info_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Author".to_vec(), Object::string_literal("AI Robot")),
            (b"Title".to_vec(), Object::string_literal("Generated")),
        ])));
        doc.trailer.set(b"Info", Object::Reference(info_id));

        let mut meta_dict = Dictionary::new();
        meta_dict.set(b"Type", Object::Name(b"Metadata".to_vec()));
        meta_dict.set(b"Subtype", Object::Name(b"XML".to_vec()));
        let meta = Stream::new(meta_dict, b"<xmp>AI generated</xmp>".to_vec());
        doc.add_object(Object::Stream(meta));

        let mut pdf = Vec::new();
        doc.save_to(&mut pdf).unwrap();

        let stripped = strip_pdf_metadata(&pdf).expect("strip ok");
        assert!(!stripped.is_empty());

        let reparsed = Document::load_mem(&stripped).unwrap();
        assert!(
            reparsed.trailer.get(b"Info").is_err(),
            "trailer /Info must be removed"
        );
        for (_id, object) in reparsed.objects.iter() {
            let lopdf::Object::Stream(stream) = object else {
                continue;
            };
            let is_metadata = stream
                .dict
                .get(b"Type")
                .ok()
                .and_then(|t| t.as_name().ok())
                .map(|n| n == b"Metadata")
                .unwrap_or(false);
            assert!(!is_metadata, "Metadata stream must be removed");
        }
    }

    #[test]
    fn test_docx_drops_docprops_and_customxml() {
        let zip = make_zip(&[
            ("word/document.xml", "<w:document/>"),
            ("docProps/core.xml", "<cp:coreProperties/>"),
            ("customXml/item1.xml", "<c/>"),
        ]);
        let out = strip_docx_metadata(&zip).expect("strip ok");
        let names = read_zip_names(&out);
        assert!(names.contains(&"word/document.xml".to_string()));
        assert!(!names.iter().any(|n| n.starts_with("docProps/")));
        assert!(!names.iter().any(|n| n.starts_with("customXml/")));
    }

    #[test]
    fn test_epub_drops_signatures_and_strips_opf_metadata() {
        let zip = make_zip(&[
            ("META-INF/container.xml", "<container/>"),
            ("META-INF/signatures.xml", "<signatures/>"),
            ("META-INF/encryption.xml", "<encryption/>"),
            ("META-INF/rights.xml", "<rights/>"),
            (
                "content.opf",
                "<package><metadata><dc:creator>Robot</dc:creator></metadata><manifest/></package>",
            ),
            ("chapter.xhtml", "<html/>"),
        ]);
        let out = strip_epub_metadata(&zip).expect("strip ok");
        let mut archive = ZipArchive::new(Cursor::new(out)).unwrap();
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).unwrap();
            let name = f.name().to_string();
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).unwrap();
            let text = String::from_utf8(buf).unwrap();
            if name == "content.opf" {
                assert!(
                    !text.contains("<metadata>"),
                    "OPF metadata block should be stripped, got: {}",
                    text
                );
            }
            assert!(
                !name.starts_with("META-INF/signatures")
                    && !name.starts_with("META-INF/encryption")
                    && !name.starts_with("META-INF/rights"),
                "signature/encryption/rights entries must be dropped, kept: {}",
                name
            );
        }
    }

    #[test]
    fn test_odt_drops_meta() {
        let zip = make_zip(&[
            ("content.xml", "<office:document/>"),
            ("meta.xml", "<office:meta/>"),
            ("META-INF/documentsignatures.xml", "<sigs/>"),
        ]);
        let out = strip_odt_metadata(&zip).expect("strip ok");
        let names = read_zip_names(&out);
        assert!(names.contains(&"content.xml".to_string()));
        assert!(!names.contains(&"meta.xml".to_string()));
        assert!(!names
            .iter()
            .any(|n| n.starts_with("META-INF/documentsignatures")));
    }

    #[test]
    fn test_svg_strips_metadata_comments_and_sensitive_attrs() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" data-c2pa-claim-gu="abc"><Metadata><dc:creator>AI</dc:creator></Metadata><!-- provenance --><ellipse cx="5" cy="5" data-c2pa-reference="1" fill="red"/><rect data-c2pa-bad value width="2"/></svg>"#;
        let out = String::from_utf8(strip_svg_metadata(svg.as_bytes()).unwrap()).unwrap();

        let lower = out.to_lowercase();
        assert!(
            !lower.contains("c2pa"),
            "data-c2pa-* attributes must be removed, got: {}",
            out
        );
        assert!(
            !lower.contains("<metadata"),
            "metadata blocks must be removed"
        );
        assert!(!out.contains("<!--"), "comments must be removed");
        assert!(out.contains("<svg"), "root element preserved");
        assert!(out.contains("<ellipse"), "image elements preserved");
        assert!(
            out.contains("<rect"),
            "malformed trailing attr does not break loop"
        );
        assert!(out.ends_with("</svg>"), "closing tag preserved");
    }

    #[test]
    fn test_svg_non_ascii_content_preserved() {
        let svg = "<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"><text>café 🚀</text></svg>";
        let out = String::from_utf8(strip_svg_metadata(svg.as_bytes()).unwrap()).unwrap();
        assert!(out.contains("café 🚀"));
        assert_eq!(out, svg);
    }

    #[test]
    fn test_strip_xml_metadata_block_helper() {
        let xml = "<package><metadata><dc:creator>x</dc:creator></metadata><item/></package>";
        let out = strip_xml_metadata_block(xml);
        assert!(!out.contains("<metadata>"));
        assert!(out.contains("<item/>"));
    }
}
