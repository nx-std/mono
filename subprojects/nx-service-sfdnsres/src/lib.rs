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
//! ## One session, not one per call
//!
//! A C resolver typically opens a new service session for every call (via
//! `smGetServiceOriginal` + `serviceClose`). This crate follows the
//! convention of the other `nx-service-*` crates instead: connect once via
//! [`connect_cmif`], reuse the [`SfdnsresService`] across calls, and close
//! the session explicitly with `Drop`.
//!
//! ## Decoded Results
//!
//! This crate owns the `sfdnsres` wire-format codec. Commands that
//! exchange serialized hostent / addrinfo data (cmds 2, 3, 6) and the
//! `getnameinfo` reply (cmd 7) encode their typed inputs and decode their
//! responses internally: callers pass typed inputs ([`AddrInfoHints`], an
//! `IpAddr`, a `SocketAddr`) and receive owned, structurally-valid result
//! types ([`HostEntry`], [`AddrInfoList`], [`NameInfo`]). The serialized
//! layout is `sfdnsres`-specific knowledge, so it lives beside the commands
//! that produce it rather than in a consumer crate.

#![no_std]

extern crate alloc; // String, Vec
extern crate nx_panic_handler; // Provide #![panic_handler]

use core::net::{
    IpAddr,
    SocketAddr,
};

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
pub mod netdb;
mod proto;
mod wire;

pub use self::{
    cmif::{
        CommandError,
        GetAddrInfoResult,
        GetHostByAddrResult,
        GetHostByNameResult,
        GetNameInfoResult,
    },
    proto::{
        CancelHandle,
        NameInfoFlags,
        SERVICE_NAME,
    },
    wire::{
        AddrFamily,
        AddrInfoHints,
        AddrInfoList,
        HostEntry,
        NameInfo,
        Protocol,
        ResolvedAddr,
        SockType,
        WireError,
    },
};

/// DNS Resolver Service (`sfdnsres`) session wrapper.
///
/// Provides type safety to distinguish `sfdnsres` sessions from other services.
#[repr(transparent)]
pub struct SfdnsresService(Session);

impl SfdnsresService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SfdnsresService {
    /// Resolves a host name (`GetHostByNameRequest`, cmd 2).
    #[inline]
    pub fn get_host_by_name(
        &self,
        cancel_handle: Option<CancelHandle>,
        use_nsd: bool,
        name: Option<&str>,
    ) -> Result<GetHostByNameResult, CommandError> {
        cmif::get_host_by_name(self.0.handle(), cancel_handle, use_nsd, name)
    }

    /// Reverse-resolves an IP address (`GetHostByAddrRequest`, cmd 3).
    #[inline]
    pub fn get_host_by_addr(
        &self,
        cancel_handle: Option<CancelHandle>,
        addr: IpAddr,
    ) -> Result<GetHostByAddrResult, CommandError> {
        cmif::get_host_by_addr(self.0.handle(), cancel_handle, addr)
    }

    /// Looks up the textual description of an `h_errno` value
    /// (`GetHostStringErrorRequest`, cmd 4).
    #[inline]
    pub fn get_host_string_error(&self, err: u32, out_str: &mut [u8]) -> Result<(), CommandError> {
        cmif::get_host_string_error(self.0.handle(), err, out_str)
    }

    /// Looks up the textual description of a `getaddrinfo` error code
    /// (`GetGaiStringErrorRequest`, cmd 5).
    #[inline]
    pub fn get_gai_string_error(&self, err: u32, out_str: &mut [u8]) -> Result<(), CommandError> {
        cmif::get_gai_string_error(self.0.handle(), err, out_str)
    }

    /// Performs a `getaddrinfo`-style resolution
    /// (`GetAddrInfoRequest`, cmd 6).
    #[inline]
    pub fn get_addr_info(
        &self,
        cancel_handle: Option<CancelHandle>,
        use_nsd: bool,
        node: Option<&str>,
        service: Option<&str>,
        hints: &AddrInfoHints,
    ) -> Result<GetAddrInfoResult, CommandError> {
        cmif::get_addr_info(
            self.0.handle(),
            cancel_handle,
            use_nsd,
            node,
            service,
            hints,
        )
    }

    /// Performs a `getnameinfo`-style reverse lookup
    /// (`GetNameInfoRequest`, cmd 7).
    #[inline]
    pub fn get_name_info(
        &self,
        cancel_handle: Option<CancelHandle>,
        flags: NameInfoFlags,
        addr: &SocketAddr,
    ) -> Result<GetNameInfoResult, CommandError> {
        cmif::get_name_info(self.0.handle(), cancel_handle, flags, addr)
    }

    /// Allocates a fresh cancel-token
    /// (`GetCancelHandleRequest`, cmd 8).
    #[inline]
    pub fn get_cancel_handle(&self) -> Result<CancelHandle, CommandError> {
        cmif::get_cancel_handle(self.0.handle())
    }

    /// Cancels any pending resolver call tagged with `handle`
    /// (`CancelRequest`, cmd 9).
    #[inline]
    pub fn cancel(&self, handle: CancelHandle) -> Result<(), CommandError> {
        cmif::cancel(self.0.handle(), handle)
    }
}

/// Connects to the `sfdnsres` (DNS resolver) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<SfdnsresService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(SfdnsresService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get sfdnsres service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
