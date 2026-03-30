//! Mutex abstraction: parking_lot on native, std::cell on wasm (single-threaded).

#[cfg(not(target_arch = "wasm32"))]
pub use parking_lot::Mutex;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::{RefCell, RefMut};
    use std::sync::Arc;

    /// Single-threaded mutex for wasm32 using RefCell.
    pub struct Mutex<T: ?Sized>(RefCell<T>);

    // SAFETY: wasm32-unknown-unknown is single-threaded.
    unsafe impl<T: ?Sized> Send for Mutex<T> {}
    unsafe impl<T: ?Sized> Sync for Mutex<T> {}

    impl<T> Mutex<T> {
        pub fn new(val: T) -> Self {
            Mutex(RefCell::new(val))
        }

        pub fn lock(&self) -> RefMut<'_, T> {
            self.0.borrow_mut()
        }

        pub fn try_lock(&self) -> Option<RefMut<'_, T>> {
            self.0.try_borrow_mut().ok()
        }

        pub fn lock_arc(self: &Arc<Self>) -> ArcMutexGuard<T> {
            ArcMutexGuard {
                _arc: Arc::clone(self),
                // SAFETY: single-threaded wasm — the Arc keeps the Mutex alive,
                // and we hold the borrow for the guard's lifetime.
                guard: unsafe { &*Arc::as_ptr(self) }.0.borrow_mut(),
            }
        }

        pub fn try_lock_arc(self: &Arc<Self>) -> Option<ArcMutexGuard<T>> {
            let guard = unsafe { &*Arc::as_ptr(self) }.0.try_borrow_mut().ok()?;
            Some(ArcMutexGuard {
                _arc: Arc::clone(self),
                guard,
            })
        }
    }

    /// An RAII guard that keeps an `Arc<Mutex<T>>` alive while borrowed.
    pub struct ArcMutexGuard<T: ?Sized + 'static> {
        // Must drop guard before arc, so guard is listed first.
        guard: RefMut<'static, T>,
        _arc: Arc<Mutex<T>>,
    }

    // SAFETY: wasm32 is single-threaded.
    unsafe impl<T: ?Sized> Send for ArcMutexGuard<T> {}

    impl<T: ?Sized> std::ops::Deref for ArcMutexGuard<T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.guard
        }
    }

    impl<T: ?Sized> std::ops::DerefMut for ArcMutexGuard<T> {
        fn deref_mut(&mut self) -> &mut T {
            &mut *self.guard
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::Mutex;
