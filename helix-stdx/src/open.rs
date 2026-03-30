/// Open a URL in the platform's default handler.
///
/// - Native: spawns `xdg-open` / `open` / etc. via the `open` crate.
/// - WASM: calls `globalThis.open(url, "_blank")` to open a browser tab.
///
/// Returns `true` if the URL was opened successfully.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_url(url: &str) -> bool {
    for cmd in open::commands(url) {
        let mut command: std::process::Command = cmd.into();
        if command.output().is_ok() {
            return true;
        }
    }
    false
}

#[cfg(target_arch = "wasm32")]
pub fn open_url(url: &str) -> bool {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["globalThis"], js_name = open, catch)]
        fn window_open(url: &str, target: &str) -> Result<JsValue, JsValue>;
    }

    window_open(url, "_blank")
        .map(|v| !v.is_null() && !v.is_undefined())
        .unwrap_or(false)
}
