//! Crate-internal IPC syscall wrapper binding the TLS-buffer token to the
//! send.
//!
//! The kernel call [`nx_svc::ipc::send_sync_request`] takes only a session
//! handle. Its real memory footprint travels through side channels: the
//! message it reads and overwrites lives in the TLS IPC region addressed
//! implicitly via the `TPIDRRO_EL0` system register, and the buffer
//! descriptors inside that message carry raw addresses the kernel maps,
//! reads, and writes during the call. Neither appears in the syscall's
//! type, so the compiler cannot constrain reference lifetimes against the
//! call site on its own.
//!
//! This wrapper re-declares the first side channel at the type level:
//! taking [`IpcBuffer`] by unique reference statically invalidates every
//! outstanding borrow of the TLS bytes (via [`IpcBuffer::as_array`] and
//! friends) before the kernel overwrites them, and it only reborrows the
//! caller's token, so the singleton-per-thread contract on [`ipc_buffer`]
//! is composed with, never weakened.
//!
//! The second side channel - the descriptor targets - cannot be typed at
//! this altitude: once the request is bytes in TLS, the loans that
//! justified its addresses are invisible to the type system. That
//! obligation is therefore this function's `unsafe` precondition,
//! discharged at its single call site, [`HipcRequest::send_inner`], which
//! holds every descriptor loan inside the request value it consumes across
//! the call. Keeping the wrapper `pub(crate)` makes that the only door:
//! outside this crate the syscall is reachable solely through the
//! loan-consuming `send` methods on CMIF and TIPC requests.
//!
//! [`HipcRequest::send_inner`]: crate::hipc::HipcRequest
//! [`IpcBuffer`]: nx_sys_thread_tls::IpcBuffer
//! [`IpcBuffer::as_array`]: nx_sys_thread_tls::IpcBuffer::as_array
//! [`ipc_buffer`]: nx_sys_thread_tls::ipc_buffer

pub use nx_svc::ipc::{Handle, Handle as SessionHandle, SendSyncError};
use nx_sys_thread_tls::IpcBuffer;

/// Sends a synchronous IPC request on `session`, using `buf` as the
/// request/response area.
///
/// The kernel reads the request the caller wrote into the buffer and
/// overwrites it with the response. Taking `buf` by unique reference
/// ensures no `&`/`&mut` to the underlying byte array is live across the
/// syscall; fresh borrows taken afterwards observe the response.
///
/// # Safety
///
/// Every buffer descriptor currently serialized in `buf` must point to
/// memory that stays live, correctly sized, and appropriately loaned
/// (exclusively borrowed for kernel-written roles, not mutated for
/// kernel-read roles) until this call returns. Callers uphold this by
/// holding the request value - which owns those loans - across the call.
#[inline]
pub(crate) unsafe fn send_sync_request(
    buf: &mut IpcBuffer,
    session: SessionHandle,
) -> Result<(), SendSyncError> {
    // The buffer is identified to the kernel implicitly via TPIDRRO_EL0;
    // `buf` does not need to be passed through as a parameter. Its role
    // here is purely type-level: the unique reborrow proves at the call
    // site that no aliasing borrow of the bytes can outlive the syscall.
    let _ = buf;
    // SAFETY: descriptor-target validity is this function's own `# Safety`
    // precondition, forwarded verbatim; the `&mut IpcBuffer` reborrow
    // guarantees no reference into the TLS bytes is live across the call.
    unsafe { nx_svc::ipc::send_sync_request(session) }
}
