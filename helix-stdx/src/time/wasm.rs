//! Async timer primitives for `wasm32-unknown-unknown`, backed by the browser's
//! `setTimeout` / `clearTimeout` via `js-sys`.
//!
//! Provides API-compatible replacements for `tokio::time::{sleep, timeout, Sleep}`
//! and `tokio::time::error::Elapsed` so that the rest of the codebase can use a
//! single import path (`helix_stdx::time`) regardless of target.

use super::{Duration, Instant};

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use js_sys::{Array, Function, JsString, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

/// Analogous to `tokio::time::Instant`. On wasm32 this is simply
/// `web_time::Instant` (backed by `performance.now()`).
pub type TimerInstant = Instant;

// ── setTimeout / clearTimeout helpers ───────────────────────────────────────

fn js_set_timeout(callback: &Function, millis: i32) -> i32 {
    let global = js_sys::global();
    let key: JsValue = JsString::from("setTimeout").into();
    let func: Function = Reflect::get(&global, &key)
        .expect("globalThis.setTimeout missing")
        .unchecked_into();
    let args = Array::of2(&callback.into(), &JsValue::from(millis));
    Reflect::apply(&func, &global, &args)
        .expect("setTimeout call failed")
        .as_f64()
        .unwrap_or(0.0) as i32
}

fn js_clear_timeout(id: i32) {
    let global = js_sys::global();
    let key: JsValue = JsString::from("clearTimeout").into();
    let func: Function = Reflect::get(&global, &key)
        .expect("globalThis.clearTimeout missing")
        .unchecked_into();
    let args = Array::of1(&JsValue::from(id));
    let _ = Reflect::apply(&func, &global, &args);
}

// ── Shared waker state ─────────────────────────────────────────────────────

struct SleepShared {
    waker: Cell<Option<Waker>>,
    fired: Cell<bool>,
}

fn schedule_wake(shared: &Rc<SleepShared>, millis: i32) -> (i32, Closure<dyn FnMut()>) {
    let s = Rc::clone(shared);
    let closure = Closure::wrap(Box::new(move || {
        s.fired.set(true);
        if let Some(w) = s.waker.take() {
            w.wake();
        }
    }) as Box<dyn FnMut()>);

    let id = js_set_timeout(closure.as_ref().unchecked_ref(), millis);
    (id, closure)
}

fn dur_to_millis(d: Duration) -> i32 {
    d.as_millis().min(i32::MAX as u128) as i32
}

// ── Sleep future ────────────────────────────────────────────────────────────

/// A future that completes after a deadline, analogous to `tokio::time::Sleep`.
///
/// Supports [`Pin::as_mut().reset()`](Sleep::reset) to reschedule without
/// re-allocating, matching the `tokio::time::Sleep` API used by `Editor`'s
/// `idle_timer` and `redraw_timer`.
pub struct Sleep {
    deadline: TimerInstant,
    shared: Rc<SleepShared>,
    timeout_id: Cell<Option<i32>>,
    // Must be kept alive so the closure isn't GC'd before it fires.
    _closure: Option<Closure<dyn FnMut()>>,
}

// Safety: wasm32-unknown-unknown is single-threaded; these types are never
// shared across threads.
unsafe impl Send for Sleep {}
unsafe impl Sync for Sleep {}

impl Sleep {
    fn new(deadline: TimerInstant) -> Self {
        let shared = Rc::new(SleepShared {
            waker: Cell::new(None),
            fired: Cell::new(false),
        });
        let now = TimerInstant::now();
        let millis = if deadline > now {
            dur_to_millis(deadline.duration_since(now))
        } else {
            0
        };
        let (id, closure) = schedule_wake(&shared, millis);
        Sleep {
            deadline,
            shared,
            timeout_id: Cell::new(Some(id)),
            _closure: Some(closure),
        }
    }

    /// Returns the instant at which this sleep will complete.
    pub fn deadline(&self) -> TimerInstant {
        self.deadline
    }

    /// Resets the sleep to complete at a new `deadline`.
    pub fn reset(self: Pin<&mut Self>, deadline: TimerInstant) {
        // Safety: we only mutate plain data fields, not the pinning invariant.
        let this = unsafe { self.get_unchecked_mut() };

        // Cancel the old timer.
        if let Some(id) = this.timeout_id.take() {
            js_clear_timeout(id);
        }

        this.deadline = deadline;
        this.shared.fired.set(false);

        let now = TimerInstant::now();
        let millis = if deadline > now {
            dur_to_millis(deadline.duration_since(now))
        } else {
            0
        };
        let (id, closure) = schedule_wake(&this.shared, millis);
        this.timeout_id.set(Some(id));
        this._closure = Some(closure);
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.shared.fired.get() {
            Poll::Ready(())
        } else {
            self.shared.waker.set(Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some(id) = self.timeout_id.take() {
            js_clear_timeout(id);
        }
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Maximum far-future offset: ~24.8 days (i32::MAX milliseconds).
/// Capped to avoid overflow with `web_time::Instant` which cannot represent
/// `Instant::now() + Duration::MAX`.
const FAR_FUTURE: Duration = Duration::from_millis(i32::MAX as u64);

/// Returns a [`Sleep`] future that completes after `duration`.
pub fn sleep(duration: Duration) -> Sleep {
    let now = TimerInstant::now();
    let deadline = now.checked_add(duration).unwrap_or(now + FAR_FUTURE);
    Sleep::new(deadline)
}

/// Error returned by [`timeout`] when the deadline elapses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("deadline has elapsed")
    }
}

impl std::error::Error for Elapsed {}

/// Wraps a future with a deadline: resolves to `Err(Elapsed)` if `duration`
/// elapses before `future` completes.
pub fn timeout<F: Future>(duration: Duration, future: F) -> Timeout<F> {
    Timeout {
        future,
        sleep: sleep(duration),
    }
}

/// Future returned by [`timeout`].
#[must_use = "futures do nothing unless polled"]
pub struct Timeout<F> {
    future: F,
    sleep: Sleep,
}

// Safety: same single-threaded wasm32 argument as Sleep.
unsafe impl<F: Send> Send for Timeout<F> {}
unsafe impl<F: Sync> Sync for Timeout<F> {}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: structural pin projection – both fields are pinned in place.
        let (future, sleep) = unsafe {
            let this = self.get_unchecked_mut();
            (
                Pin::new_unchecked(&mut this.future),
                Pin::new_unchecked(&mut this.sleep),
            )
        };

        // Check the inner future first (biased toward completion).
        if let Poll::Ready(v) = future.poll(cx) {
            return Poll::Ready(Ok(v));
        }
        if let Poll::Ready(()) = sleep.poll(cx) {
            return Poll::Ready(Err(Elapsed));
        }
        Poll::Pending
    }
}
