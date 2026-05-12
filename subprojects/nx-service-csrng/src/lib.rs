//! Cryptographic Secure RNG (`csrng`) service implementation.
//!
//! Provides access to the system's hardware-backed CSPRNG via a single
//! command that fills a caller-provided buffer with random bytes.
//! CMIF only — non-domain.
//!
//! ## Divergence from libnx
//!
//! libnx's `csrng.c` keeps a guarded global singleton (`g_csrngSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`CsrngService`] across calls, and close
//! the session explicitly with [`CsrngService::close`].

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;

pub use self::{cmif::GetRandomBytesError, proto::SERVICE_NAME};

/// Cryptographic Secure RNG (`csrng`) session wrapper.
#[repr(transparent)]
pub struct CsrngService(Service);

impl CsrngService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the `csrng` session.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }
}

/// CMIF protocol methods.
impl CsrngService {
    /// Fills `out` with cryptographically-secure random bytes.
    #[inline]
    pub fn get_random_bytes(&self, out: &mut [u8]) -> Result<(), GetRandomBytesError> {
        cmif::get_random_bytes(self.0.session, out)
    }
}

/// Connects to the `csrng` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<CsrngService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    Ok(CsrngService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get csrng service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
