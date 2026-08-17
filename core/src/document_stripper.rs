use lopdf::Document;
use std::io::{Cursor, Read, Write};
use zip::{ZipArchive, ZipWriter};
use zip::write::SimpleFileOptions;

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
    doc.save_to(&mut out_buffer).map_err(|e| format!("Failed to save PDF: {}", e))?;
    Ok(out_buffer)
}

pub fn strip_docx_metadata(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Failed to parse DOCX (ZIP): {}", e))?;
    
    let mut out_buffer = Vec::new();
    let cursor = Cursor::new(&mut out_buffer);
    let mut zip_writer = ZipWriter::new(cursor);
    
    let options = SimpleFileOptions::default();
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Error reading ZIP file: {}", e))?;
        let name = file.name().to_string();
        
        // Skip docProps/ and customXml/ as they contain author and provenance metadata
        if name.starts_with("docProps/") || name.starts_with("customXml/") {
            continue;
        }
        
        zip_writer.start_file(name, options.clone()).map_err(|e| format!("Error writing ZIP file: {}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| format!("Error reading file content: {}", e))?;
        zip_writer.write_all(&buffer).map_err(|e| format!("Error writing file content: {}", e))?;
    }
    
    // Convert back from mutable cursor reference to owned Vec<u8>
    let cursor = zip_writer.finish().map_err(|e| format!("Failed to finalize DOCX (ZIP): {}", e))?;
    out_buffer = cursor.into_inner().to_vec();
    
    Ok(out_buffer)
}
