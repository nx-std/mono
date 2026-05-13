//! RAII wrappers for the copy-handles returned by pm process hooks and the
//! `pm:shell` process event.
//!
//! Both types own an [`nx_svc::sync::EventHandle`] so the kernel handle is
//! closed when the wrapper is dropped, and both expose a blocking
//! [`wait`](ProcessEvent::wait) via `svcWaitSynchronization`.

use nx_svc::{raw::Handle, sync::EventHandle};

/// Owned process event handle returned by `pm:shell GetProcessEventHandle`.
///
/// The underlying event is always auto-clearing (libnx forces
/// `eventLoadRemote(..., autoclear=true)`).
#[derive(Debug)]
pub struct ProcessEvent {
    handle: EventHandle,
}

impl ProcessEvent {
    /// Adopts an event copy-handle from a successful `pm:shell` dispatch.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid event handle owned by the current process and
    /// not yet wrapped by any other RAII type. The handle is closed when the
    /// returned [`ProcessEvent`] is dropped.
    pub(crate) unsafe fn from_raw(raw: Handle) -> Self {
        Self {
            // SAFETY: forwarded by the caller.
            handle: unsafe { EventHandle::from_raw(raw) },
        }
    }

    /// Blocks until the event is signalled or `timeout_ns` elapses.
    ///
    /// Pass `u64::MAX` for an infinite wait.
    pub fn wait(&self, timeout_ns: u64) -> Result<(), WaitError> {
        // SAFETY: `self.handle` is a valid kernel handle owned by this process
        // for as long as `self` lives, and it is not one of the pseudo-handles.
        unsafe { nx_svc::sync::wait_synchronization_single(&self.handle, timeout_ns) }
            .map_err(WaitError)
    }

    /// Returns the underlying [`EventHandle`] for interop with other
    /// `nx-svc`-level primitives.
    pub fn as_event_handle(&self) -> &EventHandle {
        &self.handle
    }

    /// Returns the raw kernel handle.
    pub fn as_raw(&self) -> Handle {
        self.handle.to_raw()
    }
}

impl Drop for ProcessEvent {
    fn drop(&mut self) {
        // SAFETY: `self.handle` was acquired from a successful pm IPC dispatch,
        // is owned by this process, and is being released exactly once.
        let _ = unsafe { nx_svc::raw::close_handle(self.handle.to_raw()) };
    }
}

/// Owned process-creation hook returned by `pm:dmnt HookToCreateProcess` and
/// `HookToCreateApplicationProcess`.
#[derive(Debug)]
pub struct ProcessHook {
    handle: EventHandle,
}

impl ProcessHook {
    /// Adopts a hook copy-handle from a successful `pm:dmnt` dispatch.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid event handle owned by the current process and
    /// not yet wrapped by any other RAII type.
    pub(crate) unsafe fn from_raw(raw: Handle) -> Self {
        Self {
            // SAFETY: forwarded by the caller.
            handle: unsafe { EventHandle::from_raw(raw) },
        }
    }

    /// Blocks until the hook fires or `timeout_ns` elapses.
    ///
    /// Pass `u64::MAX` for an infinite wait.
    pub fn wait(&self, timeout_ns: u64) -> Result<(), WaitError> {
        // SAFETY: see [`ProcessEvent::wait`].
        unsafe { nx_svc::sync::wait_synchronization_single(&self.handle, timeout_ns) }
            .map_err(WaitError)
    }

    /// Returns the underlying [`EventHandle`].
    pub fn as_event_handle(&self) -> &EventHandle {
        &self.handle
    }

    /// Returns the raw kernel handle.
    pub fn as_raw(&self) -> Handle {
        self.handle.to_raw()
    }
}

impl Drop for ProcessHook {
    fn drop(&mut self) {
        // SAFETY: `self.handle` was acquired from a successful pm IPC dispatch,
        // is owned by this process, and is being released exactly once.
        let _ = unsafe { nx_svc::raw::close_handle(self.handle.to_raw()) };
    }
}

/// Failure waiting on a [`ProcessEvent`] or [`ProcessHook`].
#[derive(Debug, thiserror::Error)]
#[error("wait_synchronization failed")]
pub struct WaitError(#[source] pub nx_svc::sync::WaitSyncError);

/// Opaque cookie consumed by
/// [`DebugMonitorService::clear_hook`](crate::DebugMonitorService::clear_hook)
/// on `[6.0.0+]` to remove a previously installed hook.
///
/// The value is caller-supplied; pm does not attach a cookie to the returned
/// [`ProcessHook`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HookId(pub u32);
