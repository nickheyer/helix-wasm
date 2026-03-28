use helix_view::input::{Event, KeyCode, KeyEvent, KeyModifiers};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let mut msg = format!("PANIC: {info}");
        if let Some(location) = info.location() {
            msg.push_str(&format!(
                "\n  at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            ));
        }
        log(&msg);
    }));
    console_log!("Helix WASM module loaded");

    helix_loader::initialize_config_file(None);
    helix_loader::initialize_log_file(None);
}

/// Start the editor.
///
/// The tokio runtime is created *inside* the async block returned to the
/// browser so that it stays alive for the entire lifetime of the Promise.
/// We only enable the task scheduler (not the time driver, which panics on
/// wasm32-unknown-unknown) — all timer functionality comes from
/// `helix_stdx::time` which uses `setTimeout` under the hood.
#[wasm_bindgen]
pub fn run_editor() -> js_sys::Promise {
    console_log!("Starting Helix editor...");

    wasm_bindgen_futures::future_to_promise(async move {
        // Build a single-threaded tokio runtime (task scheduler + channels only,
        // no time driver — time is handled by helix_stdx::time via setTimeout).
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to create tokio runtime");

        // Enter the runtime context so tokio::spawn works.
        let _guard = rt.enter();

        let args = helix_term::args::Args::default();
        let config = helix_term::config::Config::default();
        let lang_loader = helix_core::config::default_lang_loader();

        let app = helix_term::application::Application::new(args, config, lang_loader)
            .map_err(|e| JsValue::from_str(&format!("Failed to create application: {e}")))?;

        console_log!("Application created, starting event loop...");

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async move {
                let mut app = app;
                let mut events = app.event_stream();
                app.run(&mut events).await.map_err(|e| {
                    JsValue::from_str(&format!("Application error: {e}"))
                })?;
                Ok(JsValue::UNDEFINED)
            })
            .await
    })
}

// ── JS → Rust event bridge ──────────────────────────────────────────────────

/// Called from JS on every `keydown`. Translates the DOM `KeyboardEvent`
/// fields into a helix `Event::Key` and pushes it into the editor's input
/// stream.
#[wasm_bindgen]
pub fn send_key_event(key: &str, shift: bool, ctrl: bool, alt: bool) {
    // Pure modifier presses don't produce editor events.
    if matches!(key, "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "NumLock" | "ScrollLock") {
        return;
    }

    let key_code = match key {
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Escape" => KeyCode::Esc,
        "Delete" => KeyCode::Delete,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Insert" => KeyCode::Insert,
        s if s.starts_with('F') && s.len() > 1 => {
            match s[1..].parse::<u8>() {
                Ok(n @ 1..=24) => KeyCode::F(n),
                _ => return,
            }
        }
        s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return,
    };

    let mut modifiers = KeyModifiers::NONE;
    if shift {
        modifiers.insert(KeyModifiers::SHIFT);
    }
    if ctrl {
        modifiers.insert(KeyModifiers::CONTROL);
    }
    if alt {
        modifiers.insert(KeyModifiers::ALT);
    }

    helix_term::wasm_input::send(Event::Key(KeyEvent {
        code: key_code,
        modifiers,
    }));
}

/// Called from JS when the terminal is resized.
#[wasm_bindgen]
pub fn send_resize_event(cols: u16, rows: u16) {
    helix_term::wasm_input::send(Event::Resize(cols, rows));
}

/// Called from JS on paste.
#[wasm_bindgen]
pub fn send_paste_event(text: &str) {
    helix_term::wasm_input::send(Event::Paste(text.to_string()));
}
