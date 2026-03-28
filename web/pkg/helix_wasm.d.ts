/* tslint:disable */
/* eslint-disable */
export function start(): void;
/**
 * Start the editor.
 *
 * The tokio runtime is created *inside* the async block returned to the
 * browser so that it stays alive for the entire lifetime of the Promise.
 * We only enable the task scheduler (not the time driver, which panics on
 * wasm32-unknown-unknown) — all timer functionality comes from
 * `helix_stdx::time` which uses `setTimeout` under the hood.
 */
export function run_editor(): Promise<any>;
/**
 * Called from JS on every `keydown`. Translates the DOM `KeyboardEvent`
 * fields into a helix `Event::Key` and pushes it into the editor's input
 * stream.
 */
export function send_key_event(key: string, shift: boolean, ctrl: boolean, alt: boolean): void;
/**
 * Called from JS when the terminal is resized.
 */
export function send_resize_event(cols: number, rows: number): void;
/**
 * Called from JS on paste.
 */
export function send_paste_event(text: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly start: () => void;
  readonly run_editor: () => any;
  readonly send_key_event: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly send_resize_event: (a: number, b: number) => void;
  readonly send_paste_event: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h6aa9a5b364d2dd0d: (a: number, b: number) => void;
  readonly closure4892_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure3985_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
