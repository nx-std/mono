//! # nx-wlaninf
//!
//! Idiomatic Rust API for the Horizon OS `wlan:inf` (WLAN InfraManager)
//! service, built on [`nx_service_wlaninf`]'s CMIF transport.
//!
//! ## What `wlan:inf` is
//!
//! `wlan:inf` is the read-only diagnostic endpoint of the Horizon `wlan`
//! sysmodule (NCA program ID `0100000000000016`). It surfaces two commands
//! — current connection state and current received-signal-strength
//! indicator (RSSI) — that callers use to display or log WLAN status
//! without going through the full `nifm` connection-establishment surface.
//! Both commands take no input and return a single 4-byte value.
//!
//! On retail consoles the `wlan` sysmodule is the Nintendo binary;
//! **Atmosphère-NX does not re-implement it**: `stratosphere/boot2` launches
//! the stock `Wlan` program (see
//! `libstratosphere/source/boot2/boot2_api.board.nintendo_nx.cpp`) and the
//! wire format is whatever Nintendo ships.
//!
//! The `wlan` sysmodule exposes several sibling endpoints, each a separate
//! service registration with its own session pool. Session counts below
//! are taken from the Ryujinx `WlanIpcServer` registration (sysmodule-side
//! limits — `sm:` may apply ACLs on top):
//!
//! | Service     | Sessions | Firmware     | Purpose                                                  |
//! |-------------|---------:|--------------|----------------------------------------------------------|
//! | `wlan:inf`  |       10 | 1.0.0–14.1.2 | **This crate.** WLAN connection state + RSSI (read-only).|
//! | `wlan:dtc`  |        4 | 6.0.0–14.1.2 | DetectManager (network detection / scanning).            |
//! | `wlan:lcl`  |       10 | 1.0.0–14.1.2 | LocalManager (local-comm / `ldn` backing).               |
//! | `wlan:lg`   |       10 | 1.0.0–14.1.2 | LocalGetFrame.                                           |
//! | `wlan:lga`  |       10 | 1.0.0–14.1.2 | LocalGetActionFrame.                                     |
//! | `wlan:sg`   |       10 | 1.0.0–14.1.2 | SocketGetFrame.                                          |
//! | `wlan:soc`  |       10 | 1.0.0–14.1.2 | SocketManager (raw socket backing).                      |
//! | `wlan`      |       30 | 15.0.0+      | Unified GeneralServiceCreator that supersedes `wlan:inf`.|
//! | `wlan:nd`   |        5 | 15.0.0+      | `sf:driver` creator.                                     |
//! | `wlan:p`    |       30 | 15.0.0+      | PrivateServiceCreator.                                   |
//!
//! Only `wlan:inf` is covered here; the other endpoints are out of scope
//! for this crate.
//!
//! ## Surface in this crate
//!
//! One service-object type for the single endpoint, plus re-exported wire
//! types and per-command errors:
//!
//! - [`WlanInfService`] — typed wrapper around the `wlan:inf` session.
//! - [`connect`] — opens a session via the supplied [`SmService`]; the
//!   returned object owns the session and disconnects on drop.
//! - [`WlanInfState`] — `NotConnected` / `Connecting` / `Connected`.
//! - [`Rssi`] — newtype over `i32` dBm; range roughly −30 dBm (strong) to
//!   −90 dBm (barely connected) on a logarithmic scale.
//! - [`GetStateError`], [`GetRssiError`] — per-command errors (CMIF
//!   dispatch failure; `GetState` additionally surfaces an
//!   `InvalidState(raw)` variant for out-of-range values).
//!
//! ## Layering
//!
//! ```text
//!   nx-wlaninf          WlanInfService (typed API)
//!     |
//!   nx-service-wlaninf  raw CMIF wrapper (WlaninfService)
//!     |
//!     libnx wlan:inf    IPC (cmd 10 GetState, cmd 12 GetRSSI)
//! ```
//!
//! ## Firmware gating
//!
//! `wlan:inf` is available on HOS **1.0.0 – 14.1.2** and is **removed in
//! HOS 15.0.0+**, where the `wlan` sysmodule was rewritten and replaces it
//! with the unified `wlan` / `wlan:nd` / `wlan:p` endpoints listed above.
//!
//! Following the convention of `nx-service-vi` / `nx-service-wlaninf`, this
//! crate is intentionally unaware of `hosversion`: the caller is
//! responsible for confirming HOS < 15.0.0 before calling [`connect`]. On
//! HOS 15+ the lookup will fail at `sm:` and surface as [`ConnectError`].
//!
//! The optional FFI surface (`feature = "ffi"`) does perform the gate
//! inside `wlaninfInitialize` to match libnx's behaviour (returns
//! `MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer)` on HOS 15+; see
//! libnx `wlaninf.c` `_wlaninfInitialize`).
//!
//! ## References
//!
//! - Switchbrew wiki: <https://switchbrew.org/wiki/WLAN_services>
//! - libnx: `src/nx/source/services/wlaninf.c` and
//!   `include/switch/services/wlaninf.h` (commands 10 / 12,
//!   `WlanInfState` enum, RSSI range comment)
//! - Atmosphère: `stratosphere/boot2` launches the stock `Wlan` program
//!   (`SystemProgramId::Wlan = 0x0100000000000016`); the sysmodule itself
//!   is not re-implemented.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_service_wlaninf::WlaninfService;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use nx_service_wlaninf::{
    GetRssiError,
    GetStateError,
    Rssi,
    WlanInfState,
};

/// Connected `wlan:inf` (WLAN InfraManager) service.
pub struct WlanInfService {
    inner: WlaninfService,
}

impl WlanInfService {
    #[inline]
    pub(crate) fn new(inner: WlaninfService) -> Self {
        Self { inner }
    }

    /// Reads the current WLAN connection state.
    #[inline]
    pub fn state(&self) -> Result<WlanInfState, GetStateError> {
        self.inner.get_state()
    }

    /// Reads the current received signal strength.
    ///
    /// On a logarithmic scale: values run from `-30` (excellent signal) to
    /// `-90` (barely connected).
    #[inline]
    pub fn rssi(&self) -> Result<Rssi, GetRssiError> {
        self.inner.get_rssi()
    }
}

/// Opens a session to `wlan:inf` (WLAN InfraManager).
///
/// Asks `sm:` for a `wlan:inf` handle over CMIF and wraps it in a
/// [`WlanInfService`] that owns the session and closes it on drop.
///
/// Callers must ensure the running firmware is **below 15.0.0**; this layer
/// does not gate. On HOS 15+, `sm:` will refuse the lookup and surface a
/// [`ConnectError`].
pub fn connect(sm: &SmService) -> Result<WlanInfService, ConnectError> {
    let inner = nx_service_wlaninf::connect_cmif(sm).map_err(ConnectError)?;
    Ok(WlanInfService::new(inner))
}

/// Error returned by [`connect`].
///
/// Wraps the underlying `sm:` lookup / session-setup failure.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to wlan:inf")]
pub struct ConnectError(#[source] pub nx_service_wlaninf::ConnectCmifError);
