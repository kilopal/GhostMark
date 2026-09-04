/* tslint:disable */
/* eslint-disable */

/**
 * Embed a watermark with custom configuration.
 * green_pct: percentage of vocabulary that is "green" (default 50)
 * bias_pct: percentage chance to swap red→green (default 100)
 */
export function embed_watermark_configured_wasm(text: string, key: bigint, green_pct: number, bias_pct: number): string;

/**
 * Embed a SynthID-style watermark into text.
 * Returns the watermarked text.
 */
export function embed_watermark_wasm(text: string, key: bigint): string;

/**
 * Run the full eval pipeline (embed → scrub → measure).
 * Returns a JSON string with before/after z-scores and fidelity.
 */
export function run_eval_wasm(text: string, key: bigint): string;

export function sanitize_text_wasm(input: string, aggressive: boolean): string;

/**
 * Score with custom configuration.
 */
export function score_watermark_configured_wasm(text: string, key: bigint, green_pct: number): string;

/**
 * Score text for watermark presence. Returns a JSON string with:
 * { "tokens": N, "green": N, "green_frac": F, "z": F, "is_hit": bool }
 */
export function score_watermark_wasm(text: string, key: bigint): string;

export function shatter_synthid_wasm(input: string): string;

export function strip_docx_metadata_wasm(raw: Uint8Array): Uint8Array;

export function strip_epub_metadata_wasm(raw: Uint8Array): Uint8Array;

export function strip_image_bytes_wasm(raw: Uint8Array): Uint8Array;

export function strip_odt_metadata_wasm(raw: Uint8Array): Uint8Array;

export function strip_pdf_metadata_wasm(raw: Uint8Array): Uint8Array;

export function strip_svg_metadata_wasm(raw: Uint8Array): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly embed_watermark_configured_wasm: (a: number, b: number, c: bigint, d: number, e: number) => [number, number];
    readonly embed_watermark_wasm: (a: number, b: number, c: bigint) => [number, number];
    readonly run_eval_wasm: (a: number, b: number, c: bigint) => [number, number];
    readonly sanitize_text_wasm: (a: number, b: number, c: number) => [number, number];
    readonly score_watermark_configured_wasm: (a: number, b: number, c: bigint, d: number) => [number, number];
    readonly score_watermark_wasm: (a: number, b: number, c: bigint) => [number, number];
    readonly shatter_synthid_wasm: (a: number, b: number) => [number, number];
    readonly strip_docx_metadata_wasm: (a: number, b: number) => [number, number, number, number];
    readonly strip_epub_metadata_wasm: (a: number, b: number) => [number, number, number, number];
    readonly strip_image_bytes_wasm: (a: number, b: number) => [number, number, number, number];
    readonly strip_odt_metadata_wasm: (a: number, b: number) => [number, number, number, number];
    readonly strip_pdf_metadata_wasm: (a: number, b: number) => [number, number, number, number];
    readonly strip_svg_metadata_wasm: (a: number, b: number) => [number, number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
