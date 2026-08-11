//! Applet Manager (AM) — re-exported from [`nx_rt_core`].
//!
//! The applet handshake is kind-agnostic: an NRO and an NSO perform the same
//! libnx-faithful per-role bring-up — only the source of the applet-type
//! value differs. Its single authoritative implementation therefore lives in
//! [`nx_rt_core::services::applet`]; this module re-exports it so the NRO FFI
//! shims and the per-service managers keep resolving `crate::services::applet`.
//!
//! The NRO sources its applet-type value at runtime from the parsed homebrew
//! loader configuration (see [`crate::env::applet_type`]). Reading it and
//! handing it to the shared handshake is the one piece that is this crate's
//! own, and it is [`init_from_env`] below.

pub use nx_rt_core::services::applet::*;
use nx_svc::process::Handle as ProcessHandle;

use crate::env;

/// Opens the Applet Manager session for the applet type the loader supplied.
///
/// This is the NRO's applet bring-up: where an NSO has its applet type fixed
/// at build time, a homebrew NRO is told its role by whoever launched it, so
/// the type is read from the parsed configuration block rather than from a
/// weak global.
///
/// A launch that named no process handle is one addressing the process it is
/// running in, which is what the pseudo handle names.
///
/// # Errors
///
/// Returns an error when the proxy could not be opened or a command in the
/// per-role bring-up was refused. Nothing is left half-open. The role itself
/// cannot fail to convert: the startup parse folds anything it does not
/// recognise into the default role.
pub fn init_from_env() -> Result<(), ConnectError> {
    let applet_type = env::applet_type().into();
    let process_handle = env::own_process_handle().unwrap_or_else(ProcessHandle::current_process);

    init(applet_type, process_handle)
}
