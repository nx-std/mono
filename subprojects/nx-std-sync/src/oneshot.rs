//! A blocking, single-producer, single-consumer one-shot channel.
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::{condvar::Condvar, mutex::Mutex};

/// Creates a new one-shot channel for sending single values.
///
/// The returned [`Sender`] and [`Receiver`] are linked to each other.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Shared::new());
    let sender = Sender {
        inner: inner.clone(),
    };
    let receiver = Receiver { inner };
    (sender, receiver)
}

/// The sending-half of a one-shot channel.
///
/// This half can be used to send a single value to the receiver.
pub struct Sender<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// A value can only be sent if the `Receiver` has not yet been dropped.
    /// If the `Receiver` has been dropped, `Err` is returned with the value that
    /// was sent.
    pub fn send(self, value: T) -> Result<(), SendError<T>> {
        if self.inner.is_closed.load(Ordering::SeqCst) {
            return Err(SendError(value));
        }

        let mut lock = self.inner.mutx.lock();

        if self.inner.is_closed.load(Ordering::SeqCst) {
            return Err(SendError(value));
        }

        *lock = Some(value);

        self.inner.cvar.notify_one();
        Ok(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let value_guard = self.inner.mutx.lock();
        if value_guard.is_none() {
            self.inner.is_closed.store(true, Ordering::SeqCst);
            self.inner.cvar.notify_one();
        }
    }
}

/// The receiving-half of a one-shot channel.
///
/// This half can be used to receive a single value from the sender.
pub struct Receiver<T> {
    inner: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    /// Waits for a value to be sent on this channel.
    ///
    /// This function will block the current thread until a value is sent or
    /// the `Sender` is dropped.
    pub fn recv(self) -> Result<T, RecvError> {
        let mut lock = self.inner.mutx.lock();

        loop {
            if let Some(value) = lock.take() {
                return Ok(value);
            }

            if self.inner.is_closed.load(Ordering::SeqCst) {
                return Err(RecvError);
            }

            lock = self.inner.cvar.wait(lock);
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.is_closed.store(true, Ordering::SeqCst);
    }
}

/// An error returned from the `send` function on a `Sender`.
///
/// This error is returned when the receiving half of a channel is dropped.
/// The error contains the value that was attempted to be sent.
#[derive(PartialEq, Eq, Clone, Copy, thiserror::Error)]
#[error("channel closed")]
pub struct SendError<T>(pub T);

/// An error returned from the `recv` function on a `Receiver`.
///
/// This error is returned when the sending half of a channel is dropped before
/// a value is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("channel closed")]
pub struct RecvError;

/// The internal shared state of the one-shot channel.
struct Shared<T> {
    mutx: Mutex<Option<T>>,
    cvar: Condvar,
    is_closed: AtomicBool,
}

impl<T> Shared<T> {
    fn new() -> Self {
        Self {
            mutx: Mutex::new(None),
            cvar: Condvar::new(),
            is_closed: AtomicBool::new(false),
        }
    }
}
