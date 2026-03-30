// Minimal libc stubs for tree-sitter's C code running in WASM.
// These functions are imported by the WASM module as "env.*".
// They are also used by grammar .wasm side-modules (compiled with
// wasm-ld --shared) that import libc symbols.

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

export function _get_wasm_exports() {
    return wasmExports;
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
    if (!ptr) return; // C standard: free(NULL) is a no-op
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
    // C standard: realloc(NULL, size) == malloc(size)
    if (!ptr) return malloc(size);
    // C standard: realloc(ptr, 0) == free(ptr), return NULL
    if (!size) { free(ptr); return 0; }

    const oldSize = allocSizes.get(ptr) || size;
    if (wasmExports?.__wbindgen_realloc) {
        const newPtr = wasmExports.__wbindgen_realloc(ptr, oldSize, size, 1);
        allocSizes.delete(ptr);
        if (newPtr) allocSizes.set(newPtr, size);
        return newPtr;
    }
    const newPtr = malloc(size);
    if (newPtr && wasmExports?.memory) {
        const mem = new Uint8Array(wasmExports.memory.buffer);
        mem.copyWithin(newPtr, ptr, ptr + Math.min(oldSize, size));
    }
    free(ptr);
    return newPtr;
}

export function abort() {
    throw new Error("C abort() called");
}

// ── String helpers ────────────────────────────────────────────────────────

// Read a C string (null-terminated) from WASM memory.
export function readCString(ptr) {
    if (!ptr || !wasmExports?.memory) return "";
    const mem = new Uint8Array(wasmExports.memory.buffer);
    let end = ptr;
    while (mem[end] !== 0 && end < mem.length) end++;
    return new TextDecoder().decode(mem.subarray(ptr, end));
}

export function strlen(s) {
    if (!s || !wasmExports?.memory) return 0;
    const mem = new Uint8Array(wasmExports.memory.buffer);
    let len = 0;
    while (mem[s + len] !== 0) len++;
    return len;
}

export function strcmp(s1, s2) {
    if (!wasmExports?.memory) return 0;
    const mem = new Uint8Array(wasmExports.memory.buffer);
    let i = 0;
    while (true) {
        const a = mem[s1 + i], b = mem[s2 + i];
        if (a !== b) return a < b ? -1 : 1;
        if (a === 0) return 0;
        i++;
    }
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

// ── Memory operations ─────────────────────────────────────────────────────

export function memcpy(dest, src, n) {
    if (!wasmExports?.memory || !n) return dest;
    const mem = new Uint8Array(wasmExports.memory.buffer);
    mem.copyWithin(dest, src, src + n);
    return dest;
}

export function memmove(dest, src, n) {
    // copyWithin handles overlapping regions correctly.
    return memcpy(dest, src, n);
}

export function memset(dest, c, n) {
    if (!wasmExports?.memory || !n) return dest;
    new Uint8Array(wasmExports.memory.buffer, dest, n).fill(c & 0xff);
    return dest;
}

export function memcmp(s1, s2, n) {
    if (!wasmExports?.memory) return 0;
    const mem = new Uint8Array(wasmExports.memory.buffer);
    for (let i = 0; i < n; i++) {
        const a = mem[s1 + i], b = mem[s2 + i];
        if (a !== b) return a < b ? -1 : 1;
    }
    return 0;
}

// ── Character classification ──────────────────────────────────────────────

export function iswspace(c) {
    try { return /\s/.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function iswalnum(c) {
    try { return /[\p{L}\p{N}]/u.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function iswalpha(c) {
    try { return /[\p{L}]/u.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function iswdigit(c) {
    return (c >= 0x30 && c <= 0x39) ? 1 : 0;
}

export function isdigit(c) {
    return (c >= 0x30 && c <= 0x39) ? 1 : 0;
}

export function isalpha(c) {
    return ((c >= 0x41 && c <= 0x5a) || (c >= 0x61 && c <= 0x7a)) ? 1 : 0;
}

export function isalnum(c) {
    return (isalpha(c) || isdigit(c)) ? 1 : 0;
}

export function iswlower(c) {
    try { return /[\p{Ll}]/u.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function iswupper(c) {
    try { return /[\p{Lu}]/u.test(String.fromCodePoint(c)) ? 1 : 0; }
    catch { return 0; }
}

export function towupper(c) {
    try { return String.fromCodePoint(c).toUpperCase().codePointAt(0) || c; }
    catch { return c; }
}

export function towlower(c) {
    try { return String.fromCodePoint(c).toLowerCase().codePointAt(0) || c; }
    catch { return c; }
}

// ── C stdio stubs ─────────────────────────────────────────────────────────
// Format-string substitution isn't feasible from JS (varargs are raw stack
// values), so we log the format string as-is.

export function printf(fmt) {
    console.log("[C printf]", readCString(fmt));
    return 0;
}

export function fprintf(_stream, fmt) {
    console.log("[C fprintf]", readCString(fmt));
    return 0;
}

export function fputs(s, _stream) {
    console.log("[C fputs]", readCString(s));
    return 0;
}

export function snprintf(_buf, _size, _fmt) {
    return 0;
}

export function vsnprintf(_buf, _size, _fmt, _va_list) {
    return 0;
}

// ── Time ──────────────────────────────────────────────────────────────────

export function clock_gettime(_clk_id, tp) {
    if (!wasmExports?.memory) return -1;
    const now = performance.now();
    const view = new DataView(wasmExports.memory.buffer);
    view.setInt32(tp, Math.floor(now / 1000), true);
    view.setInt32(tp + 4, Math.floor((now % 1000) * 1_000_000), true);
    return 0;
}
