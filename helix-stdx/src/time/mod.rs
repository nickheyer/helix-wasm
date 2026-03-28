//! Cross-platform time types and async timer primitives.
//!
//! **Sync types** (`Instant`, `SystemTime`, `Duration`): On native platforms,
//! re-exports from `std::time`. On wasm32, `Instant` and `SystemTime` come from
//! `web-time` (backed by `performance.now()`); `Duration` is always `std::time`.
//!
//! **Async timer primitives** (`sleep`, `timeout`, `Sleep`, `Elapsed`): On native
//! platforms, delegates to `tokio::time`. On wasm32, uses browser `setTimeout` via
//! `js-sys` / `wasm-bindgen`.

// ── Sync time types ─────────────────────────────────────────────────────────

pub use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Instant, SystemTime};

#[cfg(target_arch = "wasm32")]
pub use web_time::{Instant, SystemTime};

// ── Async timer primitives ──────────────────────────────────────────────────

#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
mod native {
    pub use tokio::time::error::Elapsed;
    pub use tokio::time::Instant as TimerInstant;
    pub use tokio::time::{sleep, timeout, Sleep};
}

#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
