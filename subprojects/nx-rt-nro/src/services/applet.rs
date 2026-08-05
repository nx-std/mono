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
use nx_service_applet::AppletType;
use nx_svc::process::Handle as ProcessHandle;

use crate::env;

/// Opens the Applet Manager session for the applet type the loader supplied.
///
/// This is the NRO's `appletInitialize`: where an NSO has its applet type
/// fixed at build time, a homebrew NRO is told its role by whoever launched
/// it, so the type is read from the parsed configuration block rather than
/// from a weak global.
///
/// # Errors
///
/// Returns [`InitFromEnvError::UnknownAppletType`] when the configuration
/// block named a role this workspace has no handshake for, and
/// [`InitFromEnvError::Connect`] when the handshake itself failed.
pub fn init_from_env() -> Result<(), InitFromEnvError> {
    let raw = env::applet_type().as_raw();

    // The loader reports the role as an unsigned entry, and the service client
    // takes the signed C enum. Every value either side names is small and
    // positive, so the cast is a change of spelling.
    let Some(applet_type) = AppletType::from_raw(raw as i32) else {
        return Err(InitFromEnvError::UnknownAppletType(raw));
    };

    let process_handle = env::own_process_handle()
        .map(|handle| {
            // SAFETY: a handle from `env::own_process_handle()` is one the
            // loader supplied for this process.
            ProcessHandle::from_raw_unchecked(handle.to_raw())
        })
        .unwrap_or_else(ProcessHandle::current_process);

    init(applet_type, process_handle).map_err(InitFromEnvError::Connect)
}

/// Error returned by [`init_from_env`].
#[derive(Debug, thiserror::Error)]
pub enum InitFromEnvError {
    /// The configuration block named a role with no handshake behind it.
    ///
    /// Occurs when the loader supplies an applet-type value outside the set
    /// the service client knows. No session was opened.
    #[error("the loader supplied an unknown applet type ({0})")]
    UnknownAppletType(u32),

    /// The Applet Manager handshake failed.
    ///
    /// Occurs when the proxy could not be opened or a command in the per-role
    /// bring-up was refused. Nothing is left half-open.
    #[error("failed to open the Applet Manager session")]
    Connect(#[source] ConnectError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for InitFromEnvError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        match self {
            // libnx has no code for a role it cannot name, because its applet
            // type comes from a global it defined itself and cannot be
            // unknown. This borrows the generic failure instead.
            Self::UnknownAppletType(_) => nx_rt_core::ffi::common::GENERIC_ERROR,
            Self::Connect(err) => err.to_rc(),
        }
    }
}
