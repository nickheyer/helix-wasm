//! Channel-based event injection for the wasm32 target.
//!
//! JavaScript sends keyboard / resize / paste events through exported
//! `wasm_bindgen` functions (in the `helix-wasm` crate). Those call
//! [`send`] which pushes into an unbounded channel.  The receiving end is
//! returned by [`init`] and plugged into `Application::event_stream`.

use helix_view::input::Event;
use once_cell::sync::OnceCell;
use tokio::sync::mpsc;

static EVENT_TX: OnceCell<mpsc::UnboundedSender<Event>> = OnceCell::new();

/// Create the event channel. Returns the receiver to be used as the
/// application's input stream. Must only be called once.
pub fn init() -> mpsc::UnboundedReceiver<Event> {
    let (tx, rx) = mpsc::unbounded_channel();
    EVENT_TX
        .set(tx)
        .expect("wasm event channel already initialized");
    rx
}

/// Push an event into the channel. Called from JS via `helix-wasm` exports.
pub fn send(event: Event) {
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(event);
    }
}
