// Minimal libc stubs for tree-sitter's C code running in WASM.
// These functions are imported by the WASM module as "env.*".

// Memory management: delegates to WASM's own allocator via exported functions.
// The wasm-bindgen generated module exports __wbindgen_malloc / __wbindgen_free.
// We proxy to those, but they're not available at import time, so we lazily resolve them.
//
// __wbindgen_realloc(ptr, old_size, new_size, align) requires the original
// allocation size.  C's realloc(ptr, new_size) doesn't provide it, so we
// track sizes ourselves.
let wasmExports = null;
const allocSizes = new Map();

export function _set_wasm_exports(exports) {
    wasmExports = exports;
}

export function malloc(size) {
    if (wasmExports?.__wbindgen_malloc) {
        const ptr = wasmExports.__wbindgen_malloc(size, 1);
        if (ptr) allocSizes.set(ptr, size);
        return ptr;
    }
    return 0;
}

export function free(ptr) {
    const size = allocSizes.get(ptr) || 0;
    allocSizes.delete(ptr);
    if (wasmExports?.__wbindgen_free) {
        wasmExports.__wbindgen_free(ptr, size, 1);
    }
}

export function calloc(count, size) {
    const total = count * size;
    const ptr = malloc(total);
    if (ptr && wasmExports?.memory) {
        new Uint8Array(wasmExports.memory.buffer, ptr, total).fill(0);
    }
    return ptr;
}

export function realloc(ptr, size) {
    const oldSize = allocSizes.get(ptr) || 0;
    if (wasmExports?.__wbindgen_realloc) {
        const newPtr = wasmExports.__wbindgen_realloc(ptr, oldSize, size, 1);
        allocSizes.delete(ptr);
        if (newPtr) allocSizes.set(newPtr, size);
        return newPtr;
    }
    const newPtr = malloc(size);
    if (ptr && newPtr && wasmExports?.memory) {
        const mem = new Uint8Array(wasmExports.memory.buffer);
        mem.copyWithin(newPtr, ptr, ptr + Math.min(oldSize, size));
    }
    if (ptr) free(ptr);
    return newPtr;
}

export function abort() {
    throw new Error("C abort() called");
}

// String formatting stubs - tree-sitter uses these for debug logging only
export function fprintf(_stream, _fmt) {
    return 0;
}

export function snprintf(_buf, _size, _fmt) {
    return 0;
}

export function vsnprintf(_buf, _size, _fmt, _va_list) {
    return 0;
}

export function strncmp(s1, s2, n) {
    if (!wasmExports?.memory) return 0;
    const mem = new Uint8Array(wasmExports.memory.buffer);
    for (let i = 0; i < n; i++) {
        const a = mem[s1 + i], b = mem[s2 + i];
        if (a !== b) return a < b ? -1 : 1;
        if (a === 0) return 0;
    }
    return 0;
}

export function clock_gettime(_clk_id, tp) {
    if (!wasmExports?.memory) return -1;
    const now = performance.now();
    const view = new DataView(wasmExports.memory.buffer);
    view.setInt32(tp, Math.floor(now / 1000), true);
    view.setInt32(tp + 4, Math.floor((now % 1000) * 1_000_000), true);
    return 0;
}

export function iswspace(c) {
    try { return /\s/.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function iswalnum(c) {
    try { return /[\p{L}\p{N}]/u.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}
