/* tslint:disable */
/* eslint-disable */

export class WasmClientRuntime {
    free(): void;
    [Symbol.dispose](): void;
    ack_render_frame(tick: bigint): void;
    apply_delta(data: Uint8Array): void;
    apply_snapshot(data: Uint8Array): void;
    connect_client(_url: string): void;
    create_ack_payload(acked_tick: bigint): Uint8Array;
    create_hello_payload(): Uint8Array;
    create_input_payload(sequence: number, x: number, y: number): Uint8Array;
    disconnect_client(): void;
    extract_render_frame(): any;
    get_client_state(): number;
    get_diagnostics(): string;
    initialize_client(): void;
    constructor();
    push_input(sequence: number, x: number, y: number): void;
    push_network_message(data: Uint8Array): void;
    tick_client(delta_seconds: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmclientruntime_free: (a: number, b: number) => void;
    readonly wasmclientruntime_new: () => number;
    readonly wasmclientruntime_initialize_client: (a: number, b: number) => void;
    readonly wasmclientruntime_connect_client: (a: number, b: number, c: number, d: number) => void;
    readonly wasmclientruntime_disconnect_client: (a: number) => void;
    readonly wasmclientruntime_push_network_message: (a: number, b: number, c: number, d: number) => void;
    readonly wasmclientruntime_push_input: (a: number, b: number, c: number, d: number) => void;
    readonly wasmclientruntime_tick_client: (a: number, b: number, c: number) => void;
    readonly wasmclientruntime_apply_snapshot: (a: number, b: number, c: number, d: number) => void;
    readonly wasmclientruntime_apply_delta: (a: number, b: number, c: number, d: number) => void;
    readonly wasmclientruntime_extract_render_frame: (a: number) => number;
    readonly wasmclientruntime_ack_render_frame: (a: number, b: bigint) => void;
    readonly wasmclientruntime_create_input_payload: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly wasmclientruntime_create_hello_payload: (a: number, b: number) => void;
    readonly wasmclientruntime_create_ack_payload: (a: number, b: number, c: bigint) => void;
    readonly wasmclientruntime_get_client_state: (a: number) => number;
    readonly wasmclientruntime_get_diagnostics: (a: number, b: number) => void;
    readonly rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
    readonly rust_zstd_wasm_shim_malloc: (a: number) => number;
    readonly rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
    readonly rust_zstd_wasm_shim_free: (a: number) => void;
    readonly rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
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
