/* tslint:disable */
/* eslint-disable */
export function run_web(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly run_web: () => void;
  readonly wasm_bindgen__convert__closures_____invoke__h1537bf3c17afa922: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h091cf3d9058d15a1: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h508bdfd09cd1c406: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__ha184240bdf921b2b: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h4eeef7ab910827de: (a: number, b: number) => void;
  readonly wasm_bindgen__closure__destroy__h33446336e0438a36: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h85be312af902fb74: (a: number, b: number, c: any, d: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h5d5f8b967705b6aa: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h036ad88f5133e231: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
