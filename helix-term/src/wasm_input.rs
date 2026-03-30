//! Event injection for the wasm32 target.
//!
//! We avoid `tokio::sync::mpsc` because on wasm32 its internal `RefCell`
//! is still borrowed during `send()` when the waker fires, causing a
//! double-borrow panic when the executor re-polls `recv()`.

use helix_view::input::Event;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures_util::Stream;

thread_local! {
    static QUEUE: RefCell<VecDeque<Event>> = RefCell::new(VecDeque::new());
    static WAKER: RefCell<Option<Waker>> = RefCell::new(None);
}

/// Push an event into the queue. Called from JS via `helix-wasm` exports.
pub fn send(event: Event) {
    QUEUE.with(|q| q.borrow_mut().push_back(event));
    // Borrow is released before waking — this is the whole point.
    WAKER.with(|w| {
        if let Some(waker) = w.borrow().as_ref() {
            waker.wake_by_ref();
        }
    });
}

/// Stream that drains the event queue.
pub struct WasmEventStream;

pub fn init() -> WasmEventStream {
    WasmEventStream
}

impl Stream for WasmEventStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let event = QUEUE.with(|q| q.borrow_mut().pop_front());
        if let Some(event) = event {
            Poll::Ready(Some(event))
        } else {
            WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}
