//! # nx-service-ldn
//!
//! Rust port of libnx's `ldn` (Local Discovery Network) service surface. The
//! crate exposes the full IPC contract used by Splatoon / Mario Kart / Smash
//! / etc. for ad-hoc local networking:
//!
//! - [`LdnService`] — `ldn:u` / `ldn:s` LocalCommunicationService client,
//!   including the optional `[18.0.0+]` `IClientProcessMonitor` sub-object
//!   and the libnx-style 3-session domain pool that prevents long-running
//!   commands like `Scan` / `RecvActionFrame` from serialising unrelated
//!   state queries.
//! - [`LdnMonitorService`] — `ldn:m` IMonitorService client. Read-only
//!   subset of LCS, used by overlays/system monitors.
//! - [`connect_cmif`] / [`connect_monitor_cmif`] — set up the session(s),
//!   `ConvertToDomain` where applicable, and return a fully wired service
//!   handle. Neither function sends the libnx-style `Initialize` — that is
//!   a separate, explicit caller step (see [`LdnService::lcs_initialize_legacy`]
//!   and friends) because the cmd to invoke is hosversion-dependent.
//!
//! ## Hosversion handling
//!
//! Following the convention of [`nx-service-wlaninf`] and [`nx-service-vi`],
//! this crate is **intentionally unaware of `hosversion`**. Every libnx
//! `ldn*` IPC entry point is exposed as a method here regardless of HOS
//! gating, and the caller — typically `nx-rt` — is responsible for:
//!
//! - Picking the right `lcs_initialize_*` variant per HOS version /
//!   service kind.
//! - Skipping [`LdnService::open_client_process_monitor`] on pre-`[18.0.0]`.
//! - Choosing between [`LdnService::send_action_frame_legacy`] /
//!   [`LdnService::send_action_frame`] (and the matching `recv_*` /
//!   `set_home_channel_*`) based on `hosversionBefore(20,0,0)`.
//! - Translating dispatch errors back to libnx `IncompatSysVer`-style
//!   result codes for FFI parity.
//!
//! ## Divergence from libnx
//!
//! libnx keeps `g_ldnSrv` / `g_ldnmSrv` as guarded global singletons managed
//! by `NX_GENERATE_SERVICE_GUARD`. This crate follows the rest of the
//! `nx-service-*` family: each [`LdnService`] / [`LdnMonitorService`] is an
//! independent value the caller drives explicitly.
//!
//! libnx also closes the creator's domain object id after sub-object setup
//! (`_ldnObjectClose(&g_ldnSrvCreator)`). We skip the explicit
//! `Close(domain-object)` request — the kernel destroys the entire domain
//! when the underlying session is closed, so dropping the pool sessions
//! achieves the same end state without the extra round-trips.
//!
//! ## Scope
//!
//! This crate is the **Rust API only**. The FFI surface
//! (`__nx_service_ldn__*` symbols, `ldn_override.ld`, `nx-std` re-export)
//! is intentionally deferred to a follow-up PR.

#![no_std]

// `alloc` is needed for the session pool's `Box<[Domain]>`.
extern crate alloc;
// `nx_panic_handler` provides `#[panic_handler]`.
extern crate nx_panic_handler as _;

use alloc::{boxed::Box, vec::Vec};

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, Domain, DomainObject, Session, clone_current_object};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod dispatch;
mod proto;
mod session;
pub mod types;

use nx_sf::service::DispatchError;

use crate::{
    cmif::{creator, lcs, monitor},
    session::{LDN_POOL_SIZE, SessionPool},
};
pub use crate::{
    cmif::{
        creator::{CreateClientProcessMonitorError, CreateServiceError},
        lcs::{
            GetDisconnectReasonError, GetStateChangeEventError, GetStateError as LcsGetStateError,
            RecvActionFrameOut, channel_band_to_channel, channel_to_band, channel_to_old_band,
        },
        monitor::GetStateError as MonitorGetStateError,
    },
    proto::{
        LDN_PRIORITY_SYSTEM, LDN_PRIORITY_USER, LdnAcceptPolicy, LdnDisconnectReason,
        LdnOperationMode, LdnProtocol, LdnScanFilterFlag, LdnSecurityMode, LdnServiceType,
        LdnState, LdnWirelessControllerRestriction, SERVICE_NAME_MONITOR, SERVICE_NAME_SYSTEM,
        SERVICE_NAME_USER,
    },
};

//
// `LdnService` — `ldn:u` / `ldn:s`
//

/// Connected `ldn:u` / `ldn:s` LocalCommunicationService.
///
/// Holds the converted-to-domain creator, the LCS sub-object id, an optional
/// ICPM sub-object id, and a 3-session pool for concurrent IPC. Dropping the
/// service closes all pool sessions; the kernel tears down the domain when its
/// last session goes away, so the LCS / ICPM / creator domain objects are
/// released implicitly.
pub struct LdnService {
    kind: LdnServiceType,
    pool: SessionPool,
    lcs_object_id: u32,
    icpm_object_id: Option<u32>,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `nx_std_sync::Mutex` / `Condvar` based pool. Concurrent IPC calls from
// different threads acquire distinct pool slots, so no thread-unsafe mutation
// is performed via shared `&self`.
unsafe impl Send for LdnService {}
unsafe impl Sync for LdnService {}

impl LdnService {
    /// Returns the service flavour (`User` / `System`).
    #[inline]
    pub fn kind(&self) -> LdnServiceType {
        self.kind
    }

    /// Returns whether the `IClientProcessMonitor` sub-object has been opened.
    #[inline]
    pub fn has_client_process_monitor(&self) -> bool {
        self.icpm_object_id.is_some()
    }

    /// Acquires a pool slot, opens a `DomainObject` view onto the LCS
    /// sub-object on that slot, runs `f`, then releases the slot.
    #[inline]
    fn dispatch_lcs<R>(&self, f: impl FnOnce(&DomainObject<'_>) -> R) -> R {
        let g = self.pool.acquire();
        let obj = g
            .open_object_raw(self.lcs_object_id)
            .expect("lcs object id validated at connect_cmif");
        f(&obj)
    }

    /// Dispatches on the ICPM sub-object. Returns `Err` if the caller never
    /// called [`Self::open_client_process_monitor`].
    #[inline]
    fn dispatch_icpm<R>(
        &self,
        f: impl FnOnce(&DomainObject<'_>) -> R,
    ) -> Result<R, IcpmNotOpenedError> {
        let object_id = self.icpm_object_id.ok_or(IcpmNotOpenedError)?;
        let g = self.pool.acquire();
        let obj = g
            .open_object_raw(object_id)
            .expect("icpm object id validated at open_client_process_monitor");
        Ok(f(&obj))
    }

    /// Dispatches on the *creator* domain root via a pool slot.
    #[inline]
    fn dispatch_creator<R>(&self, f: impl FnOnce(&Domain) -> R) -> R {
        let g = self.pool.acquire();
        f(g.domain())
    }

    //
    // Initialize / Finalize variants — caller picks per hosversion.
    //

    /// `Initialize` (cmd 400) — pre-`[7.0.0]` path. `send_pid` + zero payload.
    pub fn lcs_initialize_legacy(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::initialize_legacy)
    }

    /// `InitializeWithVersion` — cmd 402 on `ldn:u` / cmd 403 on `ldn:s`.
    ///
    /// Caller must be on `[7.0.0+]`.
    pub fn lcs_initialize_with_version(&self, version: i32) -> Result<(), DispatchError> {
        let kind = self.kind;
        self.dispatch_lcs(|obj| lcs::initialize_with_version(obj, kind, version))
    }

    /// `InitializeWithPriority` (cmd 404) — `ldn:s`-only, `[19.0.0+]`.
    /// Returns [`LcsInitializeWithPriorityError::WrongKind`] if invoked on
    /// `LdnServiceType::User` (the command is not defined there).
    pub fn lcs_initialize_with_priority(
        &self,
        version: i32,
        priority: i32,
    ) -> Result<(), LcsInitializeWithPriorityError> {
        if self.kind != LdnServiceType::System {
            return Err(LcsInitializeWithPriorityError::WrongKind);
        }
        self.dispatch_lcs(|obj| lcs::initialize_with_priority(obj, version, priority))
            .map_err(LcsInitializeWithPriorityError::Dispatch)
    }

    /// `Finalize` (cmd 401).
    pub fn lcs_finalize(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::finalize)
    }

    //
    // Read commands.
    //

    /// `GetState` (cmd 0).
    pub fn get_state(&self) -> Result<LdnState, LcsGetStateError> {
        self.dispatch_lcs(lcs::get_state)
    }

    /// `GetNetworkInfo` (cmd 1).
    pub fn get_network_info(
        &self,
        out: &mut crate::types::LdnNetworkInfo,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::get_network_info(obj, out))
    }

    /// `GetIpv4Address` (cmd 2).
    pub fn get_ipv4_address(
        &self,
    ) -> Result<(crate::types::LdnIpv4Address, crate::types::LdnSubnetMask), DispatchError> {
        self.dispatch_lcs(lcs::get_ipv4_address)
    }

    /// `GetDisconnectReason` (cmd 3).
    pub fn get_disconnect_reason(&self) -> Result<LdnDisconnectReason, GetDisconnectReasonError> {
        self.dispatch_lcs(lcs::get_disconnect_reason)
    }

    /// `GetSecurityParameter` (cmd 4).
    pub fn get_security_parameter(
        &self,
    ) -> Result<crate::types::LdnSecurityParameter, DispatchError> {
        self.dispatch_lcs(lcs::get_security_parameter)
    }

    /// `GetNetworkConfig` (cmd 5).
    pub fn get_network_config(&self) -> Result<crate::types::LdnNetworkConfig, DispatchError> {
        self.dispatch_lcs(lcs::get_network_config)
    }

    /// `GetStateChangeEvent` (cmd 100). Returns a kernel handle to a
    /// caller-owned autoclear event copy.
    pub fn get_state_change_event(&self) -> Result<SessionHandle, GetStateChangeEventError> {
        self.dispatch_lcs(lcs::get_state_change_event)
    }

    /// `GetNetworkInfoAndHistory` (cmd 101).
    pub fn get_network_info_and_history(
        &self,
        network_info: &mut crate::types::LdnNetworkInfo,
        nodes: &mut [crate::types::LdnNodeLatestUpdate],
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::get_network_info_and_history(obj, network_info, nodes))
    }

    /// `Scan` (cmd 102). Returns the count of network entries the server wrote.
    pub fn scan(
        &self,
        channel: i16,
        filter: &crate::types::LdnScanFilter,
        out: &mut [crate::types::LdnNetworkInfo],
    ) -> Result<i32, DispatchError> {
        self.dispatch_lcs(|obj| lcs::scan(obj, channel, filter, out))
    }

    /// `ScanPrivate` (cmd 103).
    pub fn scan_private(
        &self,
        channel: i16,
        filter: &crate::types::LdnScanFilter,
        out: &mut [crate::types::LdnNetworkInfo],
    ) -> Result<i32, DispatchError> {
        self.dispatch_lcs(|obj| lcs::scan_private(obj, channel, filter, out))
    }

    //
    // Write commands.
    //

    /// `SetWirelessControllerRestriction` (cmd 104, `[5.0.0+]`).
    pub fn set_wireless_controller_restriction(
        &self,
        restriction: LdnWirelessControllerRestriction,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_wireless_controller_restriction(obj, restriction))
    }

    /// `SetProtocol` (cmd 106, `[18.0.0+]`).
    pub fn set_protocol(&self, protocol: LdnProtocol) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_protocol(obj, protocol))
    }

    /// `OpenAccessPoint` (cmd 200).
    pub fn open_access_point(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::open_access_point)
    }

    /// `CloseAccessPoint` (cmd 201).
    pub fn close_access_point(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::close_access_point)
    }

    /// `CreateNetwork` (cmd 202).
    pub fn create_network(
        &self,
        sec_config: &crate::types::LdnSecurityConfig,
        user_config: &crate::types::LdnUserConfig,
        network_config: &crate::types::LdnNetworkConfig,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::create_network(obj, sec_config, user_config, network_config))
    }

    /// `CreateNetworkPrivate` (cmd 203).
    #[allow(clippy::too_many_arguments)]
    pub fn create_network_private(
        &self,
        sec_config: &crate::types::LdnSecurityConfig,
        sec_param: &crate::types::LdnSecurityParameter,
        user_config: &crate::types::LdnUserConfig,
        network_config: &crate::types::LdnNetworkConfig,
        addrs: &[crate::types::LdnAddressEntry],
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| {
            lcs::create_network_private(
                obj,
                sec_config,
                sec_param,
                user_config,
                network_config,
                addrs,
            )
        })
    }

    /// `DestroyNetwork` (cmd 204).
    pub fn destroy_network(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::destroy_network)
    }

    /// `Reject` (cmd 205).
    pub fn reject(&self, addr: crate::types::LdnIpv4Address) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::reject(obj, addr))
    }

    /// `SetAdvertiseData` (cmd 206). Pass `&[]` to reset.
    pub fn set_advertise_data(&self, data: &[u8]) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_advertise_data(obj, data))
    }

    /// `SetStationAcceptPolicy` (cmd 207).
    pub fn set_station_accept_policy(&self, policy: LdnAcceptPolicy) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_station_accept_policy(obj, policy))
    }

    /// `AddAcceptFilterEntry` (cmd 208).
    pub fn add_accept_filter_entry(
        &self,
        addr: crate::types::LdnMacAddress,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::add_accept_filter_entry(obj, addr))
    }

    /// `ClearAcceptFilter` (cmd 209).
    pub fn clear_accept_filter(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::clear_accept_filter)
    }

    /// `OpenStation` (cmd 300).
    pub fn open_station(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::open_station)
    }

    /// `CloseStation` (cmd 301).
    pub fn close_station(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::close_station)
    }

    /// `Connect` (cmd 302).
    pub fn connect(
        &self,
        sec_config: &crate::types::LdnSecurityConfig,
        user_config: &crate::types::LdnUserConfig,
        version: i32,
        option: u32,
        network_info: &crate::types::LdnNetworkInfo,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| {
            lcs::connect(obj, sec_config, user_config, version, option, network_info)
        })
    }

    /// `ConnectPrivate` (cmd 303).
    #[allow(clippy::too_many_arguments)]
    pub fn connect_private(
        &self,
        sec_config: &crate::types::LdnSecurityConfig,
        sec_param: &crate::types::LdnSecurityParameter,
        user_config: &crate::types::LdnUserConfig,
        version: i32,
        option: u32,
        network_config: &crate::types::LdnNetworkConfig,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| {
            lcs::connect_private(
                obj,
                sec_config,
                sec_param,
                user_config,
                version,
                option,
                network_config,
            )
        })
    }

    /// `Disconnect` (cmd 304).
    pub fn disconnect(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::disconnect)
    }

    /// `SetOperationMode` — cmd 402 on `ldn:s` / cmd 403 on `ldn:u`.
    pub fn set_operation_mode(&self, mode: LdnOperationMode) -> Result<(), DispatchError> {
        let kind = self.kind;
        self.dispatch_lcs(|obj| lcs::set_operation_mode(obj, kind, mode))
    }

    //
    // `[18.0.0+]` ActionFrame family.
    //

    /// `EnableActionFrame` (cmd 500).
    pub fn enable_action_frame(
        &self,
        settings: &crate::types::LdnActionFrameSettings,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::enable_action_frame(obj, settings))
    }

    /// `DisableActionFrame` (cmd 501).
    pub fn disable_action_frame(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::disable_action_frame)
    }

    /// `SendActionFrame` (cmd 502) — pre-`[20.0.0]` ABI.
    pub fn send_action_frame_legacy(
        &self,
        data: &[u8],
        destination: crate::types::LdnMacAddress,
        bssid: crate::types::LdnMacAddress,
        channel: i16,
        flags: u32,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| {
            lcs::send_action_frame_legacy(obj, data, destination, bssid, channel, flags)
        })
    }

    /// `SendActionFrame` (cmd 502) — `[20.0.0+]` packed-band ABI.
    pub fn send_action_frame(
        &self,
        data: &[u8],
        destination: crate::types::LdnMacAddress,
        bssid: crate::types::LdnMacAddress,
        channel: i16,
        flags: u32,
    ) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| {
            lcs::send_action_frame(obj, data, destination, bssid, channel, flags)
        })
    }

    /// `RecvActionFrame` (cmd 503) — pre-`[20.0.0]` ABI.
    pub fn recv_action_frame_legacy(
        &self,
        data: &mut [u8],
        flags: u32,
    ) -> Result<RecvActionFrameOut, DispatchError> {
        self.dispatch_lcs(|obj| lcs::recv_action_frame_legacy(obj, data, flags))
    }

    /// `RecvActionFrame` (cmd 503) — `[20.0.0+]` packed-band ABI.
    pub fn recv_action_frame(
        &self,
        data: &mut [u8],
        flags: u32,
    ) -> Result<RecvActionFrameOut, DispatchError> {
        self.dispatch_lcs(|obj| lcs::recv_action_frame(obj, data, flags))
    }

    /// `SetHomeChannel` (cmd 505) — pre-`[20.0.0]` ABI.
    pub fn set_home_channel_legacy(&self, channel: i16) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_home_channel_legacy(obj, channel))
    }

    /// `SetHomeChannel` (cmd 505) — `[20.0.0+]` packed-band ABI.
    pub fn set_home_channel(&self, channel: i16) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_home_channel(obj, channel))
    }

    /// `SetTxPower` (cmd 600).
    pub fn set_tx_power(&self, power: i16) -> Result<(), DispatchError> {
        self.dispatch_lcs(|obj| lcs::set_tx_power(obj, power))
    }

    /// `ResetTxPower` (cmd 601).
    pub fn reset_tx_power(&self) -> Result<(), DispatchError> {
        self.dispatch_lcs(lcs::reset_tx_power)
    }

    //
    // `IClientProcessMonitor` (`[18.0.0+]`).
    //

    /// Opens the `IClientProcessMonitor` sub-object via creator-cmd-1.
    /// Caller must be on `[18.0.0+]`.
    pub fn open_client_process_monitor(&mut self) -> Result<(), OpenClientProcessMonitorError> {
        let id = self
            .dispatch_creator(creator::create_client_process_monitor)
            .map_err(OpenClientProcessMonitorError::Dispatch)?;
        self.icpm_object_id = Some(id);
        Ok(())
    }

    /// `RegisterClient` (ICPM cmd 0) — requires
    /// [`Self::open_client_process_monitor`] to have been called.
    pub fn icpm_register_client(&self) -> Result<(), IcpmCallError> {
        self.dispatch_icpm(crate::cmif::icpm::register_client)
            .map_err(|_| IcpmCallError::NotOpened)?
            .map_err(IcpmCallError::Dispatch)
    }
}

/// Connects to `ldn:u` / `ldn:s`. Sets up the domain conversion, the 3-session
/// pool, and creates the LCS sub-object. Does **not** send any `Initialize`
/// nor open the `IClientProcessMonitor` — both are explicit follow-up calls
/// because they are hosversion-gated.
pub fn connect_cmif(sm: &SmService, kind: LdnServiceType) -> Result<LdnService, ConnectCmifError> {
    // 1. Look up the creator service.
    let creator_handle = sm
        .get_service_handle_cmif(kind.service_name())
        .map_err(ConnectCmifError::GetService)?;

    // 2. Wrap as a Session (queries pointer-buffer size internally).
    let creator_session = Session::new(creator_handle);
    let pointer_buffer_size = creator_session.pointer_buffer_size();

    // 3. Convert to domain. On failure the Session is returned alongside
    //    the error so its Drop closes the kernel handle.
    let creator = creator_session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    // 4. Clone the creator session N-1 times; slot 0 of the pool is the
    //    original creator session, slots 1..N are clones. All N sessions
    //    are kernel-level clones of the same server-side domain.
    let mut sessions: Vec<Domain> = Vec::with_capacity(LDN_POOL_SIZE);
    sessions.push(creator);
    for _ in 1..LDN_POOL_SIZE {
        // SAFETY: cloning a domain session yields another kernel handle that
        // addresses the same domain object table on the server side, and the
        // pointer-buffer size is shared across clones.
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        let cloned_domain =
            unsafe { Domain::from_handle_unchecked(cloned_handle, pointer_buffer_size) };
        sessions.push(cloned_domain);
    }

    // 5. CreateUser/SystemLocalCommService (creator cmd 0) — dispatched on
    //    the root domain handle of slot 0.
    let lcs_object_id =
        creator::create_service_domain(&sessions[0]).map_err(ConnectCmifError::CreateService)?;

    let pool = SessionPool::new(sessions.into_boxed_slice() as Box<[Domain]>);

    Ok(LdnService {
        kind,
        pool,
        lcs_object_id,
        icpm_object_id: None,
    })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `ldn:u` / `ldn:s` failed.
    #[error("failed to look up ldn:u/ldn:s service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the creator session to a domain failed.
    #[error("failed to ConvertToDomain on ldn:u/ldn:s creator")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Cloning the creator session to fill the pool failed.
    #[error("failed to clone ldn creator session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
    /// `CreateUserLocalCommService` / `CreateSystemLocalCommService` failed.
    #[error("failed to create LocalCommunicationService sub-object")]
    CreateService(#[source] CreateServiceError),
}

/// Errors returned by [`LdnService::open_client_process_monitor`].
#[derive(Debug, thiserror::Error)]
pub enum OpenClientProcessMonitorError {
    /// IPC dispatch failed. Most commonly: caller invoked on pre-`[18.0.0]`.
    #[error("failed to dispatch CreateClientProcessMonitor")]
    Dispatch(#[source] CreateClientProcessMonitorError),
}

/// Errors returned by [`LdnService::lcs_initialize_with_priority`].
#[derive(Debug, thiserror::Error)]
pub enum LcsInitializeWithPriorityError {
    /// Method called on `LdnServiceType::User`; the command is `ldn:s`-only.
    #[error("InitializeWithPriority is only defined for LdnServiceType::System")]
    WrongKind,
    /// IPC dispatch failed.
    #[error("failed to dispatch InitializeWithPriority")]
    Dispatch(#[source] DispatchError),
}

/// Errors returned by ICPM dispatch helpers on [`LdnService`].
#[derive(Debug, thiserror::Error)]
pub enum IcpmCallError {
    /// Caller never invoked [`LdnService::open_client_process_monitor`].
    #[error("IClientProcessMonitor has not been opened")]
    NotOpened,
    /// IPC dispatch failed.
    #[error("failed to dispatch ICPM call")]
    Dispatch(#[source] DispatchError),
}

/// Marker error returned by [`LdnService::dispatch_icpm`] when the caller
/// hasn't opened the ICPM sub-object yet.
struct IcpmNotOpenedError;

//
// `LdnMonitorService` — `ldn:m`
//

/// Connected `ldn:m` IMonitorService (read-only state monitor).
///
/// libnx's `ldn:m` flow does **not** convert to domain — the IMonitorService
/// is a fresh standalone session returned by `CreateMonitorService` (cmd 0).
/// Dropping the service closes the IMonitorService session.
pub struct LdnMonitorService {
    monitor: Session,
}

// SAFETY: `Session` is just a kernel-handle wrapper.
unsafe impl Send for LdnMonitorService {}
unsafe impl Sync for LdnMonitorService {}

impl LdnMonitorService {
    /// `InitializeMonitor` (cmd 100).
    pub fn initialize_monitor(&self) -> Result<(), DispatchError> {
        monitor::initialize_monitor(&self.monitor)
    }

    /// `FinalizeMonitor` (cmd 101).
    pub fn finalize_monitor(&self) -> Result<(), DispatchError> {
        monitor::finalize_monitor(&self.monitor)
    }

    /// `GetState` (cmd 0).
    pub fn get_state(&self) -> Result<LdnState, MonitorGetStateError> {
        monitor::get_state(&self.monitor)
    }

    /// `GetNetworkInfo` (cmd 1).
    pub fn get_network_info(
        &self,
        out: &mut crate::types::LdnNetworkInfo,
    ) -> Result<(), DispatchError> {
        monitor::get_network_info(&self.monitor, out)
    }

    /// `GetIpv4Address` (cmd 2).
    pub fn get_ipv4_address(
        &self,
    ) -> Result<(crate::types::LdnIpv4Address, crate::types::LdnSubnetMask), DispatchError> {
        monitor::get_ipv4_address(&self.monitor)
    }

    /// `GetSecurityParameter` (cmd 4).
    pub fn get_security_parameter(
        &self,
    ) -> Result<crate::types::LdnSecurityParameter, DispatchError> {
        monitor::get_security_parameter(&self.monitor)
    }

    /// `GetNetworkConfig` (cmd 5).
    pub fn get_network_config(&self) -> Result<crate::types::LdnNetworkConfig, DispatchError> {
        monitor::get_network_config(&self.monitor)
    }
}

/// Connects to `ldn:m` and returns the IMonitorService child session. Does
/// **not** send `InitializeMonitor` (cmd 100) — the caller decides.
pub fn connect_monitor_cmif(sm: &SmService) -> Result<LdnMonitorService, ConnectMonitorCmifError> {
    // 1. Look up the creator service.
    let creator_handle = sm
        .get_service_handle_cmif(SERVICE_NAME_MONITOR)
        .map_err(ConnectMonitorCmifError::GetService)?;

    let creator = Session::new(creator_handle);

    // 2. CreateMonitorService (cmd 0 on the non-domain creator) returns the
    //    new IMonitorService session as a move handle.
    let monitor_handle = creator::create_service_session(&creator)
        .map_err(ConnectMonitorCmifError::CreateService)?;

    // 3. Drop the creator session — we hold the new IMonitorService session.
    drop(creator);

    // 4. Wrap the new session, querying pointer-buffer-size on it.
    let monitor = Session::new(monitor_handle);

    Ok(LdnMonitorService { monitor })
}

/// Errors returned by [`connect_monitor_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectMonitorCmifError {
    /// SM lookup for `ldn:m` failed.
    #[error("failed to look up ldn:m service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// `CreateMonitorService` failed.
    #[error("failed to create IMonitorService")]
    CreateService(#[source] CreateServiceError),
}
