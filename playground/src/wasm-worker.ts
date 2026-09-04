import init, { 
    sanitize_text_wasm, 
    shatter_synthid_wasm,
    strip_image_bytes_wasm,
    strip_pdf_metadata_wasm, 
    strip_docx_metadata_wasm, 
    strip_epub_metadata_wasm, 
    strip_odt_metadata_wasm, 
    strip_svg_metadata_wasm,
    embed_watermark_wasm,
    embed_watermark_configured_wasm,
    score_watermark_wasm,
    score_watermark_configured_wasm,
    run_eval_wasm
} from './pkg/ghostmark_wasm';

let wasmInitialized = false;

self.onmessage = async (e: MessageEvent) => {
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
        else if (action === 'shatter_synthid_text') {
            const cleanedText = shatter_synthid_wasm(payload);
            self.postMessage({ id, success: true, payload: cleanedText });
        }
        else if (action === 'embed_watermark') {
            const { text, key, greenPct, biasPct } = payload;
            const watermarked = greenPct !== undefined 
                ? embed_watermark_configured_wasm(text, key, greenPct, biasPct)
                : embed_watermark_wasm(text, key);
            self.postMessage({ id, success: true, payload: watermarked });
        }
        else if (action === 'score_watermark') {
            const { text, key, greenPct } = payload;
            const result = greenPct !== undefined
                ? score_watermark_configured_wasm(text, key, greenPct)
                : score_watermark_wasm(text, key);
            self.postMessage({ id, success: true, payload: JSON.parse(result) });
        }
        else if (action === 'run_eval') {
            const { text, key } = payload;
            const report = JSON.parse(run_eval_wasm(text, key));
            self.postMessage({ id, success: true, payload: report });
        }
        else if (action === 'strip_file') {
            const inputBytes = new Uint8Array(payload);
            let cleanedBytes: Uint8Array;

            const ext = fileName.toLowerCase().split('.').pop();

            if (['png', 'jpeg', 'jpg', 'webp', 'bmp', 'gif'].includes(ext || '')) {
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

            // Transfer the buffer back to the main thread with zero-copy
            // Note: TypeScript strict might complain about postMessage signatures, so we cast to any or just pass it
            (self as any).postMessage(
                { id, success: true, payload: cleanedBytes.buffer }, 
                [cleanedBytes.buffer]
            );
        }
    } catch (error) {
        self.postMessage({ id, success: false, error: (error as Error).message });
    }
};
