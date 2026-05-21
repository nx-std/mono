//! IPC syscall wrappers that statically enforce the buffer-aliasing contract.
//!
//! The low-level kernel call [`nx_svc::ipc::send_sync_request`] takes only a
//! session handle; the buffer it reads and overwrites is the TLS IPC region
//! addressed implicitly via the `TPIDRRO_EL0` system register. From the
//! caller's perspective that buffer mutates "out of band" — the compiler
//! sees no parameter, no return value, and nothing to constrain reference
//! lifetimes against the syscall site.
//!
//! This module wraps that call in a signature that *does* constrain it:
//!
//! ```text
//! pub fn send_sync_request(buf: &mut IpcBuffer, session: Handle) -> Result<(), _>
//! ```
//!
//! Threading the [`IpcBuffer`] token through the wrapper turns three
//! soundness obligations into borrow-checker guarantees:
//!
//! 1. **No `&[u8; N]` / `&mut [u8; N]` to the IPC buffer survives the
//!    syscall.** Reborrowing the token uniquely (`&mut IpcBuffer`) at the
//!    call site statically invalidates any outstanding borrow obtained via
//!    [`IpcBuffer::as_array`], [`IpcBuffer::as_array_mut`], or the
//!    `Deref`/`DerefMut` impls. Whatever stability promise such a borrow
//!    carried ends before the kernel writes the buffer.
//!
//! 2. **The token itself may legally cross the syscall.** Its body is a
//!    `NonNull` to an `UnsafeCell<[u8; N]>` (see [`IpcBuffer`] for the
//!    full model). Neither carries an aliasing/stability promise about
//!    the underlying bytes, so the kernel's store is in tension with no
//!    Rust reference.
//!
//! 3. **Singleton-per-thread is preserved.** The wrapper does not
//!    construct a token; it only reborrows the caller's. The
//!    `unsafe fn`-guarded contract on [`ipc_buffer`] remains the single
//!    source of token creation, and the wrapper composes with it
//!    without weakening the invariant.
//!
//! Calling [`nx_svc::ipc::send_sync_request`] directly is still possible —
//! and still sound when no IPC-buffer borrow is live at the call site —
//! but prefer this wrapper anywhere the IPC marshaling layer has a token
//! in hand. It makes the previously documentation-only invariant
//! mechanically enforced.
//!
//! [`IpcBuffer`]: nx_sys_thread_tls::IpcBuffer
//! [`IpcBuffer::as_array`]: nx_sys_thread_tls::IpcBuffer::as_array
//! [`IpcBuffer::as_array_mut`]: nx_sys_thread_tls::IpcBuffer::as_array_mut
//! [`ipc_buffer`]: nx_sys_thread_tls::ipc_buffer

pub use nx_svc::ipc::{Handle, Handle as SessionHandle, SendSyncError};
use nx_sys_thread_tls::IpcBuffer;

/// Sends a synchronous IPC request on `session`, using `buf` as the
/// request/response area.
///
/// The kernel reads the request the caller wrote into the buffer and
/// overwrites it with the response. Taking `buf` by unique reference
/// ensures no `&`/`&mut` to the underlying byte array is live across the
/// syscall — see the module-level docs for the soundness rationale.
///
/// After this call returns, the caller may obtain fresh borrows of the
/// (now response-populated) buffer via [`IpcBuffer::as_array`] etc.
#[inline]
pub fn send_sync_request(buf: &mut IpcBuffer, session: SessionHandle) -> Result<(), SendSyncError> {
    // The buffer is identified to the kernel implicitly via TPIDRRO_EL0;
    // `buf` does not need to be passed through as a parameter. Its role
    // here is purely type-level: the unique reborrow proves at the call
    // site that no aliasing borrow of the bytes can outlive the syscall.
    let _ = buf;
    nx_svc::ipc::send_sync_request(session)
}
