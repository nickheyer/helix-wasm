//! Wasm-safe channel abstraction.
//!
//! On native: re-exports `tokio::sync::mpsc` and `tokio_stream` types directly.
//! On wasm32: custom single-threaded implementation that avoids the `RefCell`
//! double-borrow panic in tokio's mpsc (the borrow on the queue is always
//! released before waking the peer, which is the whole point).

// ── Native re-exports ───────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};

#[cfg(not(target_arch = "wasm32"))]
pub mod error {
    pub use tokio::sync::mpsc::error::{SendError, TrySendError};
}

#[cfg(not(target_arch = "wasm32"))]
pub use tokio_stream::wrappers::UnboundedReceiverStream;

// ── Wasm implementation ─────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    // ── Error types ─────────────────────────────────────────────────────

    pub mod error {
        /// Error returned when sending fails because the receiver was dropped.
        pub struct SendError<T>(pub T);

        impl<T> std::fmt::Debug for SendError<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("SendError").finish_non_exhaustive()
            }
        }

        impl<T> std::fmt::Display for SendError<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "channel closed")
            }
        }

        impl<T: std::fmt::Debug> std::error::Error for SendError<T> {}

        /// Error returned by `try_send`.
        pub enum TrySendError<T> {
            /// Channel is full.
            Full(T),
            /// Receiver was dropped.
            Closed(T),
        }

        impl<T> std::fmt::Debug for TrySendError<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Full(_) => write!(f, "Full(..)"),
                    Self::Closed(_) => write!(f, "Closed(..)"),
                }
            }
        }
    }

    // ── Shared inner state ──────────────────────────────────────────────

    /// Shared state for both bounded and unbounded channels.  Each piece of
    /// mutable state lives in its own `RefCell` so that a borrow on the queue
    /// is always released before we touch any waker, eliminating the
    /// double-borrow panic that `tokio::sync::mpsc` triggers on wasm32.
    struct Inner<T> {
        queue: RefCell<VecDeque<T>>,
        recv_waker: RefCell<Option<Waker>>,
        send_waker: RefCell<Option<Waker>>,
        rx_closed: Cell<bool>,
        sender_count: Cell<usize>,
        capacity: usize, // usize::MAX for unbounded
    }

    // Safety: wasm32-unknown-unknown is single-threaded; these types are
    // never shared across threads.  Same justification as
    // `helix-stdx/src/time/wasm.rs:90-91`.
    unsafe impl<T> Send for Inner<T> {}
    unsafe impl<T> Sync for Inner<T> {}

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Pop from the queue and wake the send-side if it was waiting.
    fn pop_and_wake_sender<T>(inner: &Inner<T>) -> Option<T> {
        let item = inner.queue.borrow_mut().pop_front();
        if item.is_some() {
            // Queue borrow is released — safe to wake now.
            if let Some(w) = inner.send_waker.borrow_mut().take() {
                w.wake();
            }
        }
        item
    }

    /// Push to the queue and wake the recv-side.
    fn push_and_wake_receiver<T>(inner: &Inner<T>, value: T) {
        inner.queue.borrow_mut().push_back(value);
        // Queue borrow is released — safe to wake now.
        if let Some(w) = inner.recv_waker.borrow_mut().take() {
            w.wake();
        }
    }

    // ── Bounded channel ─────────────────────────────────────────────────

    pub struct Sender<T> {
        inner: Rc<Inner<T>>,
    }

    impl<T> std::fmt::Debug for Sender<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Sender").finish_non_exhaustive()
        }
    }

    unsafe impl<T> Send for Sender<T> {}
    unsafe impl<T> Sync for Sender<T> {}

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            self.inner
                .sender_count
                .set(self.inner.sender_count.get() + 1);
            Sender {
                inner: Rc::clone(&self.inner),
            }
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let count = self.inner.sender_count.get() - 1;
            self.inner.sender_count.set(count);
            if count == 0 {
                // All senders gone — wake recv so it returns None.
                if let Some(w) = self.inner.recv_waker.borrow_mut().take() {
                    w.wake();
                }
            }
        }
    }

    impl<T> Sender<T> {
        pub fn send(&self, value: T) -> SendFuture<T> {
            SendFuture {
                inner: Rc::clone(&self.inner),
                value: Some(value),
            }
        }

        pub fn try_send(&self, value: T) -> Result<(), error::TrySendError<T>> {
            if self.inner.rx_closed.get() {
                return Err(error::TrySendError::Closed(value));
            }
            if self.inner.queue.borrow().len() >= self.inner.capacity {
                return Err(error::TrySendError::Full(value));
            }
            push_and_wake_receiver(&self.inner, value);
            Ok(())
        }
    }

    /// Future returned by [`Sender::send`].
    pub struct SendFuture<T> {
        inner: Rc<Inner<T>>,
        value: Option<T>,
    }

    // No self-referential data — safe to unpin.
    impl<T> Unpin for SendFuture<T> {}
    unsafe impl<T> Send for SendFuture<T> {}
    unsafe impl<T> Sync for SendFuture<T> {}

    impl<T> Future for SendFuture<T> {
        type Output = Result<(), error::SendError<T>>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            if this.inner.rx_closed.get() {
                return Poll::Ready(Err(error::SendError(this.value.take().unwrap())));
            }
            if this.inner.queue.borrow().len() < this.inner.capacity {
                push_and_wake_receiver(&this.inner, this.value.take().unwrap());
                Poll::Ready(Ok(()))
            } else {
                *this.inner.send_waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub struct Receiver<T> {
        inner: Rc<Inner<T>>,
    }

    impl<T> std::fmt::Debug for Receiver<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Receiver").finish_non_exhaustive()
        }
    }

    unsafe impl<T> Send for Receiver<T> {}
    unsafe impl<T> Sync for Receiver<T> {}

    impl<T> Drop for Receiver<T> {
        fn drop(&mut self) {
            self.inner.rx_closed.set(true);
            if let Some(w) = self.inner.send_waker.borrow_mut().take() {
                w.wake();
            }
        }
    }

    impl<T> Receiver<T> {
        pub fn recv(&mut self) -> RecvFuture<'_, T> {
            RecvFuture {
                inner: &self.inner,
            }
        }
    }

    /// Future returned by [`Receiver::recv`] and [`UnboundedReceiver::recv`].
    pub struct RecvFuture<'a, T> {
        inner: &'a Rc<Inner<T>>,
    }

    impl<T> Unpin for RecvFuture<'_, T> {}
    unsafe impl<T> Send for RecvFuture<'_, T> {}
    unsafe impl<T> Sync for RecvFuture<'_, T> {}

    impl<T> Future for RecvFuture<'_, T> {
        type Output = Option<T>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(item) = pop_and_wake_sender(self.inner) {
                Poll::Ready(Some(item))
            } else if self.inner.sender_count.get() == 0 {
                Poll::Ready(None)
            } else {
                *self.inner.recv_waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let inner = Rc::new(Inner {
            queue: RefCell::new(VecDeque::new()),
            recv_waker: RefCell::new(None),
            send_waker: RefCell::new(None),
            rx_closed: Cell::new(false),
            sender_count: Cell::new(1),
            capacity,
        });
        (
            Sender {
                inner: Rc::clone(&inner),
            },
            Receiver { inner },
        )
    }

    // ── Unbounded channel ───────────────────────────────────────────────

    pub struct UnboundedSender<T> {
        inner: Rc<Inner<T>>,
    }

    impl<T> std::fmt::Debug for UnboundedSender<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("UnboundedSender").finish_non_exhaustive()
        }
    }

    unsafe impl<T> Send for UnboundedSender<T> {}
    unsafe impl<T> Sync for UnboundedSender<T> {}

    impl<T> Clone for UnboundedSender<T> {
        fn clone(&self) -> Self {
            self.inner
                .sender_count
                .set(self.inner.sender_count.get() + 1);
            UnboundedSender {
                inner: Rc::clone(&self.inner),
            }
        }
    }

    impl<T> Drop for UnboundedSender<T> {
        fn drop(&mut self) {
            let count = self.inner.sender_count.get() - 1;
            self.inner.sender_count.set(count);
            if count == 0 {
                if let Some(w) = self.inner.recv_waker.borrow_mut().take() {
                    w.wake();
                }
            }
        }
    }

    impl<T> UnboundedSender<T> {
        pub fn send(&self, value: T) -> Result<(), error::SendError<T>> {
            if self.inner.rx_closed.get() {
                return Err(error::SendError(value));
            }
            push_and_wake_receiver(&self.inner, value);
            Ok(())
        }
    }

    pub struct UnboundedReceiver<T> {
        inner: Rc<Inner<T>>,
    }

    impl<T> std::fmt::Debug for UnboundedReceiver<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("UnboundedReceiver").finish_non_exhaustive()
        }
    }

    unsafe impl<T> Send for UnboundedReceiver<T> {}
    unsafe impl<T> Sync for UnboundedReceiver<T> {}

    impl<T> Drop for UnboundedReceiver<T> {
        fn drop(&mut self) {
            self.inner.rx_closed.set(true);
        }
    }

    impl<T> UnboundedReceiver<T> {
        pub fn recv(&mut self) -> RecvFuture<'_, T> {
            RecvFuture {
                inner: &self.inner,
            }
        }
    }

    pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
        let inner = Rc::new(Inner {
            queue: RefCell::new(VecDeque::new()),
            recv_waker: RefCell::new(None),
            send_waker: RefCell::new(None),
            rx_closed: Cell::new(false),
            sender_count: Cell::new(1),
            capacity: usize::MAX,
        });
        (
            UnboundedSender {
                inner: Rc::clone(&inner),
            },
            UnboundedReceiver { inner },
        )
    }

    // ── UnboundedReceiverStream ─────────────────────────────────────────

    /// Wraps an [`UnboundedReceiver`] and implements [`futures_core::Stream`].
    pub struct UnboundedReceiverStream<T> {
        inner: UnboundedReceiver<T>,
    }

    unsafe impl<T> Send for UnboundedReceiverStream<T> {}
    unsafe impl<T> Sync for UnboundedReceiverStream<T> {}
    impl<T> Unpin for UnboundedReceiverStream<T> {}

    impl<T> UnboundedReceiverStream<T> {
        pub fn new(recv: UnboundedReceiver<T>) -> Self {
            Self { inner: recv }
        }
    }

    impl<T> futures_core::Stream for UnboundedReceiverStream<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
            let this = self.get_mut();
            if let Some(item) = pop_and_wake_sender(&this.inner.inner) {
                Poll::Ready(Some(item))
            } else if this.inner.inner.sender_count.get() == 0 {
                Poll::Ready(None)
            } else {
                *this.inner.inner.recv_waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
