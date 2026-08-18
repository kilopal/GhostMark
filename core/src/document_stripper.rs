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
    while let Some(start) = result.find("<metadata") {
        if let Some(end) = result[start..].find("</metadata>") {
            let end_abs = start + end + "</metadata>".len();
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

    // Remove data-c2pa-* attributes
    while let Some(start) = result.find("data-c2pa-") {
        // Find the attribute boundary (next quote-delimited value)
        if let Some(eq_pos) = result[start..].find('=') {
            let after_eq = start + eq_pos + 1;
            if after_eq < result.len() {
                let quote = result.as_bytes()[after_eq] as char;
                if quote == '"' || quote == '\'' {
                    if let Some(end_quote) = result[after_eq + 1..].find(quote) {
                        let end_abs = after_eq + 1 + end_quote + 1;
                        // Also remove leading whitespace before the attribute
                        let attr_start = result[..start]
                            .rfind(char::is_whitespace)
                            .unwrap_or(start);
                        result = format!("{}{}", &result[..attr_start], &result[end_abs..]);
                        continue;
                    }
                }
            }
        }
        break;
    }

    Ok(result.into_bytes())
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
