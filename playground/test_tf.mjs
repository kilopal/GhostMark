import { pipeline } from '@huggingface/transformers';

async function main() {
    const pipe = await pipeline('text-generation', 'onnx-community/Llama-3.2-1B-Instruct', {
        dtype: 'q8',
        device: 'wasm'
    });
    const chat = [
        { role: 'user', content: 'Hello' }
    ];
    const result = await pipe(chat, { max_new_tokens: 10, temperature: 0.6 });
    console.log(JSON.stringify(result, null, 2));
}

main().catch(console.error);
