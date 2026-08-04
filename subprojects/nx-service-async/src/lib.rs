//! NS/NIM IAsync sub-object wrappers.
//!
//! Provides [`AsyncValue`] and [`AsyncResult`] — typed wrappers around
//! `IAsyncValue` and `IAsyncResult` IPC sub-objects returned by services
//! such as NS and NIM.
//!
//! These are **not** standalone services: they are sub-objects that other
//! services hand out when starting asynchronous operations. There is no
//! `connect_cmif` function — callers construct instances from the handles
//! returned by the parent service via [`AsyncValue::new`] and
//! [`AsyncResult::new`].
//!
//! ## Divergence from libnx
//!
//! libnx's `async.c` gates `GetErrorContext` behind a `hosversionBefore(4,0,0)`
//! check. This crate follows the hosversion-unaware convention: the method is
//! always available, and the caller decides whether to call it based on the
//! system version.
//!
//! libnx's `asyncValueGet` / `asyncResultGet` auto-wait with `UINT64_MAX`
//! before dispatching the IPC command. This crate separates the two
//! operations: callers explicitly [`wait`](AsyncValue::wait) (or
//! [`wait`](AsyncResult::wait)), then call [`get`](AsyncValue::get) (or
//! [`get`](AsyncResult::get)). This avoids hiding a potentially long-blocking
//! wait inside a data-retrieval method.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_sf::service::{
    DispatchError,
    Session,
};
use nx_svc::sync::{
    self,
    EventHandle,
    WaitSyncError,
};

mod cmif;
mod proto;
pub mod types;

pub use self::types::{
    ErrorContext,
    ErrorContextKind,
};

/// IAsyncValue sub-object wrapper.
///
/// Holds an IPC session to an `IAsyncValue` object and an event handle
/// that signals when the async operation completes. Constructed from
/// handles returned by a parent service command.
pub struct AsyncValue {
    service: Session,
    event: EventHandle,
}

impl AsyncValue {
    /// Creates a new `AsyncValue` from a service session and event handle.
    #[inline]
    pub fn new(service: Session, event: EventHandle) -> Self {
        Self { service, event }
    }

    /// Waits for the async operation to complete or until `timeout_ns` elapses.
    ///
    /// Pass `u64::MAX` for no timeout.
    #[inline]
    pub fn wait(&self, timeout_ns: u64) -> Result<(), WaitSyncError> {
        // SAFETY: `self.event` is a valid waitable handle obtained from the
        // parent service. The kernel serializes waits on the same handle.
        unsafe { sync::wait_synchronization_single(&self.event, timeout_ns) }
    }

    /// Queries the value size (IAsyncValue cmd 0).
    #[inline]
    pub fn get_size(&self) -> Result<u64, DispatchError> {
        cmif::async_value_get_size(&self.service)
    }

    /// Retrieves the value into `buffer` (IAsyncValue cmd 1).
    ///
    /// The caller should [`wait`](Self::wait) first; this method does **not**
    /// auto-wait (diverging from libnx's `asyncValueGet`).
    #[inline]
    pub fn get(&self, buffer: &mut [u8]) -> Result<(), DispatchError> {
        cmif::async_value_get(&self.service, buffer)
    }

    /// Cancels the async operation (IAsyncValue cmd 2).
    #[inline]
    pub fn cancel(&self) -> Result<(), DispatchError> {
        cmif::async_value_cancel(&self.service)
    }

    /// Retrieves the error context (IAsyncValue cmd 3, `[4.0.0+]`).
    #[inline]
    pub fn get_error_context(&self, context: &mut ErrorContext) -> Result<(), DispatchError> {
        cmif::async_value_get_error_context(&self.service, context)
    }

    /// Cancels the operation, waits for completion, then closes both handles.
    ///
    /// Mirrors libnx's `asyncValueClose` behaviour: cancel is issued first
    /// (errors ignored), then an infinite wait, then cleanup. The session is
    /// closed when `self` is dropped at the end of this method.
    pub fn close(self) {
        let _ = self.cancel();
        let _ = self.wait(u64::MAX);
        // SAFETY: `self.event` is a valid handle obtained from the parent service.
        let _ = unsafe { nx_svc::raw::close_handle(self.event.to_raw()) };
    }
}

/// IAsyncResult sub-object wrapper.
///
/// Holds an IPC session to an `IAsyncResult` object and an event handle
/// that signals when the async operation completes. Constructed from
/// handles returned by a parent service command.
pub struct AsyncResult {
    service: Session,
    event: EventHandle,
}

impl AsyncResult {
    /// Creates a new `AsyncResult` from a service session and event handle.
    #[inline]
    pub fn new(service: Session, event: EventHandle) -> Self {
        Self { service, event }
    }

    /// Waits for the async operation to complete or until `timeout_ns` elapses.
    ///
    /// Pass `u64::MAX` for no timeout.
    #[inline]
    pub fn wait(&self, timeout_ns: u64) -> Result<(), WaitSyncError> {
        // SAFETY: `self.event` is a valid waitable handle obtained from the
        // parent service. The kernel serializes waits on the same handle.
        unsafe { sync::wait_synchronization_single(&self.event, timeout_ns) }
    }

    /// Retrieves the result code (IAsyncResult cmd 0).
    ///
    /// Returns `Ok(())` if the async operation succeeded, or a
    /// [`DispatchError`] carrying the HOS result code on failure.
    ///
    /// The caller should [`wait`](Self::wait) first; this method does **not**
    /// auto-wait (diverging from libnx's `asyncResultGet`).
    #[inline]
    pub fn get(&self) -> Result<(), DispatchError> {
        cmif::async_result_get(&self.service)
    }

    /// Cancels the async operation (IAsyncResult cmd 1).
    #[inline]
    pub fn cancel(&self) -> Result<(), DispatchError> {
        cmif::async_result_cancel(&self.service)
    }

    /// Retrieves the error context (IAsyncResult cmd 2, `[4.0.0+]`).
    #[inline]
    pub fn get_error_context(&self, context: &mut ErrorContext) -> Result<(), DispatchError> {
        cmif::async_result_get_error_context(&self.service, context)
    }

    /// Cancels the operation, waits for completion, then closes both handles.
    ///
    /// Mirrors libnx's `asyncResultClose` behaviour: cancel is issued first
    /// (errors ignored), then an infinite wait, then cleanup. The session is
    /// closed when `self` is dropped at the end of this method.
    pub fn close(self) {
        let _ = self.cancel();
        let _ = self.wait(u64::MAX);
        // SAFETY: `self.event` is a valid handle obtained from the parent service.
        let _ = unsafe { nx_svc::raw::close_handle(self.event.to_raw()) };
    }
}
