import init, { 
    sanitize_text_wasm, 
    strip_image_bytes_wasm,
    strip_pdf_metadata_wasm, 
    strip_docx_metadata_wasm, 
    strip_epub_metadata_wasm, 
    strip_odt_metadata_wasm, 
    strip_svg_metadata_wasm 
} from './pkg/ghostmark_wasm.js';

let wasmInitialized = false;

self.onmessage = async (e) => {
    const { id, action, payload, fileName } = e.data;

    try {
        if (!wasmInitialized) {
            await init();
            wasmInitialized = true;
        }

        if (action === 'sanitize_text') {
            const cleanedText = sanitize_text_wasm(payload, false);
            self.postMessage({ id, success: true, payload: cleanedText });
        } 
        else if (action === 'sanitize_text_homoglyph') {
            const cleanedText = sanitize_text_wasm(payload, true);
            self.postMessage({ id, success: true, payload: cleanedText });
        }
        else if (action === 'strip_file') {
            const inputBytes = new Uint8Array(payload);
            let cleanedBytes;
            const ext = fileName.toLowerCase().split('.').pop();

            if (['png', 'jpeg', 'jpg', 'webp', 'bmp', 'gif'].includes(ext)) {
                cleanedBytes = strip_image_bytes_wasm(inputBytes);
            } else if (ext === 'pdf') {
                cleanedBytes = strip_pdf_metadata_wasm(inputBytes);
            } else if (ext === 'docx') {
                cleanedBytes = strip_docx_metadata_wasm(inputBytes);
            } else if (ext === 'epub') {
                cleanedBytes = strip_epub_metadata_wasm(inputBytes);
            } else if (ext === 'odt') {
                cleanedBytes = strip_odt_metadata_wasm(inputBytes);
            } else if (ext === 'svg') {
                cleanedBytes = strip_svg_metadata_wasm(inputBytes);
            } else {
                throw new Error("Unsupported file type");
            }

            self.postMessage(
                { id, success: true, payload: cleanedBytes.buffer }, 
                [cleanedBytes.buffer]
            );
        }
    } catch (error) {
        self.postMessage({ id, success: false, error: error.message });
    }
};
