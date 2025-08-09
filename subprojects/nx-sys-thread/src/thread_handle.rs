//! Thread Handle Implementation
//!
//! This module contains the `Thread` struct that uses the Arc<Pin<Inner>>
//! pattern for thread handle management, following Rust's standard library approach.

use alloc::sync::Arc;
use core::pin::Pin;

use nx_svc::thread::Handle;

use crate::{thread_inner::ThreadInner, thread_stackmem::ThreadStackMem};

/// Thread handle using Arc<Pin<Inner>> pattern
#[derive(Clone)]
pub struct Thread {
    inner: Pin<Arc<ThreadInner>>,
}

impl Thread {
    /// Creates a new Thread with the Arc<Pin<Inner>> pattern
    ///
    /// This follows the same pattern as Rust's standard library for safe
    /// concurrent thread handle management.
    pub fn new(handle: Handle, stack_mem: ThreadStackMem) -> Thread {
        let inner = Arc::new(ThreadInner { handle, stack_mem });

        // SAFETY: We immediately pin the Arc after creation
        let inner = unsafe { Pin::new_unchecked(inner) };

        Thread { inner }
    }

    /// Convert Thread to raw pointer for FFI
    pub fn into_raw(self) -> *const () {
        let ptr = Arc::into_raw(unsafe { Pin::into_inner_unchecked(self.inner) });
        ptr as *const ()
    }

    /// Convert raw pointer back to Thread
    ///
    /// # Safety
    /// The pointer must have been created by `into_raw` and not used elsewhere
    pub unsafe fn from_raw(ptr: *const ()) -> Thread {
        let arc = unsafe { Arc::from_raw(ptr as *const ThreadInner) };
        let inner = unsafe { Pin::new_unchecked(arc) };
        Thread { inner }
    }

    /// Get the thread handle
    pub fn handle(&self) -> Handle {
        self.inner.handle
    }

    /// Get reference to stack memory info
    pub fn stack_mem(&self) -> &ThreadStackMem {
        &self.inner.stack_mem
    }
}
