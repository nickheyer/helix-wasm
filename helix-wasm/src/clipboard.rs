use helix_view::input::Event;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["navigator", "clipboard"], js_name = writeText)]
    fn clipboard_write(data: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = ["navigator", "clipboard"], js_name = readText)]
    async fn clipboard_read() -> Result<JsValue, JsValue>;
}

pub fn init() {
    helix_view::clipboard::register_clipboard_writer(Box::new(|text| {
        let _ = clipboard_write(text);
    }));
}

/// Async-read the system clipboard and inject the result as a paste event.
/// Called from send_key_event when Ctrl+V / Cmd+V is pressed.
pub fn paste_from_clipboard() {
    wasm_bindgen_futures::spawn_local(async {
        if let Ok(val) = clipboard_read().await {
            if let Some(text) = val.as_string() {
                if !text.is_empty() {
                    helix_term::wasm_input::send(Event::Paste(text));
                }
            }
        }
    });
}
