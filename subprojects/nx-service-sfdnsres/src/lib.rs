//! DNS Resolver Service (`sfdnsres`) implementation.
//!
//! `sfdnsres` is the Horizon OS DNS resolver — the backend for
//! `gethostbyname` / `getaddrinfo` / `getnameinfo` on Switch. This crate
//! exposes the eight resolver IPC commands as typed Rust functions.
//!
//! ## Protocol Support
//!
//! Only CMIF is implemented; TIPC is not used by any known caller of this
//! service.
//!
//! ## Divergence from libnx
//!
//! libnx's `sfdnsres.c` opens a new service session for every call (via
//! `smGetServiceOriginal` + `serviceClose`). This crate follows the
//! convention of the other `nx-service-*` crates instead: connect once via
//! [`connect_cmif`], reuse the [`SfdnsresService`] across calls, and close
//! the session explicitly with `Drop`.
//!
//! ## Output Buffers
//!
//! Commands that return serialized hostent / addrinfo data (cmds 2, 3, 6)
//! write into a caller-supplied `&mut [u8]` and report a
//! `serialized_size`. Decoding the wire format is left to a higher-level
//! consumer crate.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;

pub use self::{
    cmif::{
        CancelError, GetAddrInfoError, GetAddrInfoResult, GetCancelHandleError,
        GetGaiStringErrorError, GetHostByAddrError, GetHostByAddrResult, GetHostByNameError,
        GetHostByNameResult, GetHostStringErrorError, GetNameInfoError, GetNameInfoResult,
    },
    proto::{CancelHandle, SERVICE_NAME},
};

/// DNS Resolver Service (`sfdnsres`) session wrapper.
///
/// Provides type safety to distinguish `sfdnsres` sessions from other services.
#[repr(transparent)]
pub struct SfdnsresService(Session);

impl SfdnsresService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SfdnsresService {
    /// Resolves a host name (`GetHostByNameRequest`, cmd 2).
    ///
    /// See [`cmif::get_host_by_name`].
    #[inline]
    pub fn get_host_by_name(
        &self,
        cancel_handle: Option<CancelHandle>,
        use_nsd: bool,
        name: Option<&[u8]>,
        out_buffer: &mut [u8],
    ) -> Result<GetHostByNameResult, GetHostByNameError> {
        cmif::get_host_by_name(self.0.handle(), cancel_handle, use_nsd, name, out_buffer)
    }

    /// Reverse-resolves an address (`GetHostByAddrRequest`, cmd 3).
    #[inline]
    pub fn get_host_by_addr(
        &self,
        cancel_handle: Option<CancelHandle>,
        addr_type: u32,
        addr: &[u8],
        out_buffer: &mut [u8],
    ) -> Result<GetHostByAddrResult, GetHostByAddrError> {
        cmif::get_host_by_addr(self.0.handle(), cancel_handle, addr_type, addr, out_buffer)
    }

    /// Looks up the textual description of an `h_errno` value
    /// (`GetHostStringErrorRequest`, cmd 4).
    #[inline]
    pub fn get_host_string_error(
        &self,
        err: u32,
        out_str: &mut [u8],
    ) -> Result<(), GetHostStringErrorError> {
        cmif::get_host_string_error(self.0.handle(), err, out_str)
    }

    /// Looks up the textual description of a `getaddrinfo` error code
    /// (`GetGaiStringErrorRequest`, cmd 5).
    #[inline]
    pub fn get_gai_string_error(
        &self,
        err: u32,
        out_str: &mut [u8],
    ) -> Result<(), GetGaiStringErrorError> {
        cmif::get_gai_string_error(self.0.handle(), err, out_str)
    }

    /// Performs a `getaddrinfo`-style resolution
    /// (`GetAddrInfoRequest`, cmd 6).
    #[inline]
    pub fn get_addr_info(
        &self,
        cancel_handle: Option<CancelHandle>,
        use_nsd: bool,
        node: Option<&[u8]>,
        service: Option<&[u8]>,
        hints: Option<&[u8]>,
        out_buffer: &mut [u8],
    ) -> Result<GetAddrInfoResult, GetAddrInfoError> {
        cmif::get_addr_info(
            self.0.handle(),
            cancel_handle,
            use_nsd,
            node,
            service,
            hints,
            out_buffer,
        )
    }

    /// Performs a `getnameinfo`-style reverse lookup
    /// (`GetNameInfoRequest`, cmd 7).
    #[inline]
    pub fn get_name_info(
        &self,
        cancel_handle: Option<CancelHandle>,
        flags: u32,
        sockaddr: &[u8],
        host: &mut [u8],
        serv: &mut [u8],
    ) -> Result<GetNameInfoResult, GetNameInfoError> {
        cmif::get_name_info(self.0.handle(), cancel_handle, flags, sockaddr, host, serv)
    }

    /// Allocates a fresh cancel-token
    /// (`GetCancelHandleRequest`, cmd 8).
    #[inline]
    pub fn get_cancel_handle(&self) -> Result<CancelHandle, GetCancelHandleError> {
        cmif::get_cancel_handle(self.0.handle())
    }

    /// Cancels any pending resolver call tagged with `handle`
    /// (`CancelRequest`, cmd 9).
    #[inline]
    pub fn cancel(&self, handle: CancelHandle) -> Result<(), CancelError> {
        cmif::cancel(self.0.handle(), handle)
    }
}

/// Connects to the `sfdnsres` (DNS resolver) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<SfdnsresService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(SfdnsresService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get sfdnsres service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
