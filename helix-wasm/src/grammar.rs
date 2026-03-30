use helix_loader::grammar::GrammarLoader;
use std::collections::HashMap;
use std::sync::Mutex;
use tree_house::tree_sitter::Grammar;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Synchronously load a grammar by name.  JS fetches/caches the
    /// `grammars/<name>.wasm` file, compiles it as a shared WASM module
    /// with the main module's memory, and calls `tree_sitter_<name>()`
    /// to return the `TSLanguage*` pointer.  Returns 0 if unavailable.
    #[wasm_bindgen(js_namespace = helixGrammars, js_name = "loadSync")]
    fn load_grammar_sync(name: &str) -> u32;
}

/// Grammar loader for the WASM target.
///
/// Mirrors native `NativeGrammarLoader`:
///   native:  static registry → runtime/grammars/<name>.so  (libloading)
///   wasm:    static registry → grammars/<name>.wasm        (JS fetch + WebAssembly)
///
/// Grammar `.wasm` files are served alongside the web app and fetched by
/// JS on demand.  They are compiled as shared WASM modules (`wasm-ld
/// --shared`) that import the main module's linear memory so the returned
/// `TSLanguage*` pointer is directly usable from Rust.
pub struct WasmGrammarLoader {
    cache: Mutex<HashMap<String, Grammar>>,
}

impl WasmGrammarLoader {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl GrammarLoader for WasmGrammarLoader {
    fn load(&self, name: &str) -> anyhow::Result<Option<Grammar>> {
        if let Some(&grammar) = self.cache.lock().unwrap().get(name) {
            return Ok(Some(grammar));
        }

        // 1. Static registry (compile-time linked grammars).
        if let Some(grammar) = Grammar::from_static(name) {
            self.cache.lock().unwrap().insert(name.to_string(), grammar);
            return Ok(Some(grammar));
        }

        // 2. JS-managed: fetch from URL, compile, instantiate with shared memory.
        let ptr = load_grammar_sync(name);
        if ptr == 0 {
            return Ok(None);
        }

        let grammar = unsafe { Grammar::from_raw_ptr(ptr as usize) }
            .map_err(|e| anyhow::anyhow!("grammar '{name}': {e}"))?;
        self.cache.lock().unwrap().insert(name.to_string(), grammar);
        Ok(Some(grammar))
    }
}
