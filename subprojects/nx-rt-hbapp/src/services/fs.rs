//! `fsp-srv` service bootstrap.
//!
//! Connecting is this crate's business, because it needs the Service Manager the runtime
//! bootstraps. Holding the session afterwards is not: every filesystem, file and directory handed
//! to C is an id inside that session's domain, and so is every object the filesystem device opens,
//! so the session lives in [`nx_fsdev::service`] where both can reach it.
//!
//! What remains here is [`init`], which connects and hands the session down, and the two
//! accessors the FFI modules use, re-exported so a call site reads the same as it did when the
//! session lived here.

pub use nx_fsdev::service::{
    clear as exit,
    get as get_service,
};

use super::sm;

/// Initializes the `fsp-srv` service.
///
/// Looks the service up through the Service Manager, converts the session
/// to a domain, announces the current process, and clones the session into the request pool.
///
/// # Errors
///
/// Returns [`InitError::SmNotInitialized`] when the Service Manager has not been bootstrapped
/// yet, and [`InitError::Connect`] when `fsp-srv` could not be reached through it.
pub fn init() -> Result<(), InitError> {
    let Ok(sm) = sm::session() else {
        return Err(InitError::SmNotInitialized);
    };

    let service = nx_service_fs::connect_cmif(&sm).map_err(InitError::Connect)?;
    nx_fsdev::service::set(service);

    Ok(())
}

/// Errors returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// The Service Manager has not been bootstrapped
    ///
    /// Occurs when the filesystem is opened before the Service Manager, which is the order startup
    /// establishes. Nothing was connected and no session was installed.
    #[error("the Service Manager is not initialized")]
    SmNotInitialized,

    /// The `fsp-srv` service could not be reached
    ///
    /// Occurs when the Service Manager refused the request or the session could not be converted
    /// to a domain. Nothing was installed.
    #[error("failed to connect to the fsp-srv service")]
    Connect(#[source] nx_service_fs::ConnectCmifError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for InitError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        match self {
            // The Service Manager owns no code for "you called me too early", and libnx aborts
            // rather than reporting one, so this borrows the generic failure the caller can act on.
            Self::SmNotInitialized => nx_rt_core::ffi::common::GENERIC_ERROR,
            Self::Connect(err) => err.to_rc(),
        }
    }
}
