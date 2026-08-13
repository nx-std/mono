//! # nx-service-nifm
//!
//! Rust port of libnx's `nifm` (Network Interface Manager) service surface:
//!
//! - [`NifmService`] — `nifm:u` / `nifm:s` / `nifm:a` static-service client,
//!   plus the `IGeneralService` sub-object that hosts the bulk of the API.
//! - [`NifmRequest`] — `IRequest` sub-object created via
//!   [`NifmService::create_request`]; mirrors libnx's `NifmRequest` struct
//!   (including the two readable event handles and the cached request-state /
//!   result pair).
//! - [`connect_cmif`] — performs the SM lookup and `ConvertToDomain`. It does
//!   **not** send `CreateGeneralService` — that is a separate hosversion-gated
//!   step the caller picks via
//!   [`NifmService::open_general_service`] (cmd 5, `[3.0.0+]`, `send_pid`) or
//!   [`NifmService::open_general_service_legacy`] (cmd 4, pre-`[3.0.0]`).
//!
//! ## Hosversion handling
//!
//! Following the convention of `nx-service-ldn` and `nx-service-wlaninf`, this
//! crate is **intentionally unaware of `hosversion`**. Every libnx IPC entry
//! point is exposed regardless of HOS gating, and the caller — typically
//! `nx-rt` — is responsible for:
//!
//! - Picking [`NifmService::open_general_service_legacy`] vs.
//!   [`NifmService::open_general_service`] based on `hosversionBefore(3,0,0)`.
//! - Skipping [`NifmRequest::set_kept_in_sleep`] /
//!   [`NifmRequest::register_socket_descriptor`] /
//!   [`NifmRequest::unregister_socket_descriptor`] on pre-`[3.0.0]`.
//! - Skipping [`NifmService::set_wowl_delayed_wake_time`] on pre-`[9.0.0]`.
//! - Translating dispatch errors back to libnx `IncompatSysVer`-style result
//!   codes for FFI parity in a follow-up PR.
//!
//! ## Divergence from libnx
//!
//! libnx keeps `g_nifmSrv` / `g_nifmIGS` as guarded global singletons managed
//! by `NX_GENERATE_SERVICE_GUARD`. This crate follows the rest of the
//! `nx-service-*` family: each [`NifmService`] is an independent value the
//! caller drives explicitly. The crate also distinguishes
//! [`NifmService::set_wireless_communication_enabled`]'s
//! `NifmServiceType::User` rejection with a typed
//! [`SetWirelessCommunicationEnabledError::WrongKind`] variant instead of
//! libnx's overloaded `LibnxError_NotInitialized` result.
//!
//! ## Scope
//!
//! This crate is the **Rust API only**. The FFI surface
//! (`__nx_service_nifm__*` symbols, `nifm_override.ld`, `nx-std` re-export)
//! is intentionally deferred to a follow-up PR.

#![no_std]

extern crate nx_panic_handler as _; // Provides `#[panic_handler]`.

use core::{
    cell::Cell,
    time::Duration,
};

use nx_service_sm::SmService;
use nx_sf::service::{
    ConvertToDomainError,
    Domain,
    DomainObject,
    DomainObjectRef,
    Session,
};
use nx_svc::sync::wait_synchronization;

mod cmif;
mod dispatch;
mod proto;
pub mod types;

pub use nx_sf::service::DispatchError;
pub use nx_svc::sync::EventHandle;

use crate::{
    cmif::{
        creator,
        general,
        request,
    },
    types::{
        AppletInfo,
        InternetConnection,
        IpConfigInfo,
        NifmClientId,
        NifmIpV4Address,
        NifmNetworkProfileBasicInfo,
        NifmNetworkProfileData,
        Uuid,
    },
};
pub use crate::{
    cmif::{
        creator::CreateGeneralServiceError,
        general::{
            CreateRequestError,
            GetInternetConnectionStatusError,
        },
        request::GetSystemEventHandlesError,
    },
    proto::{
        NifmAuthentication,
        NifmEncryption,
        NifmInternetConnectionStatus,
        NifmInternetConnectionType,
        NifmNetworkProfileType,
        NifmRequestState,
        NifmServiceType,
        SERVICE_NAME_ADMIN,
        SERVICE_NAME_SYSTEM,
        SERVICE_NAME_USER,
    },
};

/// Connected `nifm:u` / `nifm:s` / `nifm:a` static service.
///
/// After [`connect_cmif`] returns, the caller must call
/// [`NifmService::open_general_service`] (or its legacy variant) before any
/// `IGeneralService` command can be issued.
pub struct NifmService {
    kind: NifmServiceType,
    /// Owning, domain-converted creator session (`nifm:*`). The
    /// `IGeneralService` sub-object lives in the same domain.
    creator: Domain,
    /// Raw domain sub-object id for `IGeneralService`. `None` until
    /// [`NifmService::open_general_service`] or
    /// [`NifmService::open_general_service_legacy`] is called. The IGS
    /// sub-object is not closed per-object; the server cascades close when
    /// the domain itself is dropped.
    igs_object_id: Option<u32>,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `Copy`/`Option<u32>`. No interior mutability is exposed via `&self`.
unsafe impl Send for NifmService {}
unsafe impl Sync for NifmService {}

impl NifmService {
    /// Returns the service flavour the session was opened against.
    #[inline]
    pub fn kind(&self) -> NifmServiceType {
        self.kind
    }

    /// Returns whether the `IGeneralService` sub-object has been opened.
    #[inline]
    pub fn has_general_service(&self) -> bool {
        self.igs_object_id.is_some()
    }

    /// `CreateGeneralServiceOld` (cmd 4, pre-`[3.0.0]`).
    pub fn open_general_service_legacy(&mut self) -> Result<(), CreateGeneralServiceError> {
        let id = creator::create_general_service_old(self.creator.as_borrowed())?;
        self.igs_object_id = Some(id);
        Ok(())
    }

    /// `CreateGeneralService` (cmd 5, `[3.0.0+]`, `send_pid`).
    pub fn open_general_service(&mut self) -> Result<(), CreateGeneralServiceError> {
        let id = creator::create_general_service(self.creator.as_borrowed())?;
        self.igs_object_id = Some(id);
        Ok(())
    }

    /// `GetClientId` (cmd 1). Errors if the general service has not been opened.
    pub fn get_client_id(&self) -> Result<NifmClientId, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::get_client_id)
    }

    /// `CreateRequest` (cmd 4). Returns a borrowed [`NifmRequest`] tied to the
    /// lifetime of `self`.
    pub fn create_request(&self) -> Result<NifmRequest<'_>, CreateRequestSurfaceError> {
        let raw_object_id =
            self.dispatch_igs(general::create_request)
                .map_err(|err| match err {
                    NotOpenedOr::NotOpened => CreateRequestSurfaceError::NotOpened,
                    NotOpenedOr::Inner(err) => CreateRequestSurfaceError::Create(err),
                })?;

        // Open the new IRequest as a borrowed domain object, then fetch its
        // two events. Dropping `object` on the error path sends the per-object
        // close request so we don't leak the sub-object.
        // SAFETY: `raw_object_id` was just returned by `cmif::create_request`
        // on this same creator domain; no other `DomainObject` references it.
        let object = DomainObject::from_raw_unchecked(self.creator.as_borrowed(), raw_object_id)
            .ok_or(CreateRequestSurfaceError::MissingObject)?;
        let (event_request_state, event1) =
            request::get_system_event_readable_handles(object.as_borrowed())
                .map_err(CreateRequestSurfaceError::GetEvents)?;

        Ok(NifmRequest {
            object,
            event_request_state,
            event1,
            // libnx initialises these to `Unknown1` / `MAKERESULT(110, 311)`;
            // we keep parity. `cached_res` uses `Err(())` as the sentinel for
            // "no result fetched yet" since the dispatch error type is opaque.
            cached_state: Cell::new(NifmRequestState::Unknown1),
            cached_res_ok: Cell::new(false),
        })
    }

    /// `GetCurrentNetworkProfile` (cmd 5).
    pub fn get_current_network_profile(
        &self,
        out: &mut NifmNetworkProfileData,
    ) -> Result<(), NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::get_current_network_profile(svc, out))
    }

    /// `EnumerateNetworkProfiles` (cmd 7). Returns the total number of
    /// profiles the server reports.
    pub fn enumerate_network_profiles(
        &self,
        kind: NifmNetworkProfileType,
        buffer: &mut [NifmNetworkProfileBasicInfo],
    ) -> Result<i32, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::enumerate_network_profiles(svc, kind, buffer))
    }

    /// `GetNetworkProfile` (cmd 8).
    pub fn get_network_profile(
        &self,
        uuid: Uuid,
        out: &mut NifmNetworkProfileData,
    ) -> Result<(), NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::get_network_profile(svc, uuid, out))
    }

    /// `SetNetworkProfile` (cmd 9). Only available with `Admin`.
    pub fn set_network_profile(
        &self,
        profile: &NifmNetworkProfileData,
    ) -> Result<Uuid, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::set_network_profile(svc, profile))
    }

    /// `GetCurrentIpAddress` (cmd 12).
    pub fn get_current_ip_address(&self) -> Result<NifmIpV4Address, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::get_current_ip_address)
    }

    /// `GetCurrentIpConfigInfo` (cmd 15).
    pub fn get_current_ip_config_info(&self) -> Result<IpConfigInfo, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::get_current_ip_config_info)
    }

    /// `SetWirelessCommunicationEnabled` (cmd 16). libnx rejects
    /// `NifmServiceType::User`; we reflect that with a typed error variant.
    pub fn set_wireless_communication_enabled(
        &self,
        enable: bool,
    ) -> Result<(), SetWirelessCommunicationEnabledError> {
        if self.kind < NifmServiceType::System {
            return Err(SetWirelessCommunicationEnabledError::WrongKind);
        }
        match self.dispatch_igs(|svc| general::set_wireless_communication_enabled(svc, enable)) {
            Ok(()) => Ok(()),
            Err(NotOpenedOr::NotOpened) => Err(SetWirelessCommunicationEnabledError::NotOpened),
            Err(NotOpenedOr::Inner(err)) => {
                Err(SetWirelessCommunicationEnabledError::Dispatch(err))
            }
        }
    }

    /// `IsWirelessCommunicationEnabled` (cmd 17).
    pub fn is_wireless_communication_enabled(&self) -> Result<bool, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::is_wireless_communication_enabled)
    }

    /// `GetInternetConnectionStatus` (cmd 18).
    pub fn get_internet_connection_status(
        &self,
    ) -> Result<InternetConnection, NotOpenedOr<GetInternetConnectionStatusError>> {
        self.dispatch_igs(general::get_internet_connection_status)
    }

    /// `IsEthernetCommunicationEnabled` (cmd 20).
    pub fn is_ethernet_communication_enabled(&self) -> Result<bool, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::is_ethernet_communication_enabled)
    }

    /// `IsAnyInternetRequestAccepted` (cmd 21).
    pub fn is_any_internet_request_accepted(
        &self,
        id: NifmClientId,
    ) -> Result<bool, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::is_any_internet_request_accepted(svc, id))
    }

    /// `IsAnyForegroundRequestAccepted` (cmd 22).
    pub fn is_any_foreground_request_accepted(&self) -> Result<bool, NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::is_any_foreground_request_accepted)
    }

    /// `PutToSleep` (cmd 23).
    pub fn put_to_sleep(&self) -> Result<(), NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::put_to_sleep)
    }

    /// `WakeUp` (cmd 24).
    pub fn wake_up(&self) -> Result<(), NotOpenedOr<DispatchError>> {
        self.dispatch_igs(general::wake_up)
    }

    /// `SetWowlDelayedWakeTime` (cmd 43, `[9.0.0+]`). The caller is responsible
    /// for gating on hosversion.
    pub fn set_wowl_delayed_wake_time(&self, val: i32) -> Result<(), NotOpenedOr<DispatchError>> {
        self.dispatch_igs(|svc| general::set_wowl_delayed_wake_time(svc, val))
    }

    /// Runs `f` against the `IGeneralService` sub-object, returning
    /// [`NotOpenedOr::NotOpened`] if [`Self::open_general_service`] (or its
    /// legacy variant) has not been called yet.
    ///
    /// The view closes nothing: the IGS is torn down by the server when the
    /// parent [`Domain`] is dropped.
    #[inline]
    fn dispatch_igs<R, E>(
        &self,
        f: impl FnOnce(DomainObjectRef<'_>) -> Result<R, E>,
    ) -> Result<R, NotOpenedOr<E>>
    where
        E: core::fmt::Debug + core::fmt::Display,
    {
        let id = self.igs_object_id.ok_or(NotOpenedOr::NotOpened)?;
        // SAFETY: `id` was returned by `create_general_service*` on this same
        // creator domain and is stored for the service's lifetime, so it names
        // a live server-side object.
        let object = DomainObjectRef::from_raw_unchecked(self.creator.as_borrowed(), id)
            .expect("IGS sub-object id is non-zero once stored");
        f(object).map_err(NotOpenedOr::Inner)
    }
}

/// Connects to the requested `nifm:*` static service via SM.
///
/// Performs the SM lookup, queries the pointer-buffer size, and converts the
/// session to a domain. The `IGeneralService` sub-object is **not** opened —
/// the caller must follow up with [`NifmService::open_general_service`]
/// or [`NifmService::open_general_service_legacy`] based on the runtime HOS
/// version.
pub fn connect_cmif(
    sm: &SmService,
    kind: NifmServiceType,
) -> Result<NifmService, ConnectCmifError> {
    // 1. Look up the static service.
    let handle = sm
        .get_service_handle_cmif(kind.service_name())
        .map_err(ConnectCmifError::GetService)?;

    // 2. Build an owned session (pointer-buffer-size query is internal).
    let session = Session::open(handle);

    // 3. Convert to a domain. On failure, drop the session.
    let creator = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(NifmService {
        kind,
        creator,
        igs_object_id: None,
    })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `nifm:u` / `nifm:s` / `nifm:a` failed.
    #[error("failed to look up nifm service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the creator session to a domain failed.
    #[error("failed to ConvertToDomain on nifm creator session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}

/// Either "the general service has not been opened" or the inner dispatch
/// error returned by an `IGeneralService` command.
#[derive(Debug, thiserror::Error)]
pub enum NotOpenedOr<E: core::fmt::Debug + core::fmt::Display> {
    /// `IGeneralService` is not opened yet — call
    /// [`NifmService::open_general_service`] or its legacy variant first.
    #[error("IGeneralService is not opened on this NifmService")]
    NotOpened,
    /// The underlying IPC dispatch failed.
    #[error("nifm IGeneralService dispatch failed")]
    Inner(#[source] E),
}

/// Error returned by [`NifmService::set_wireless_communication_enabled`].
#[derive(Debug, thiserror::Error)]
pub enum SetWirelessCommunicationEnabledError {
    /// libnx requires `NifmServiceType::System` or `NifmServiceType::Admin`;
    /// the call was made on a `NifmServiceType::User` session.
    #[error("SetWirelessCommunicationEnabled requires NifmServiceType::System or Admin")]
    WrongKind,
    /// `IGeneralService` is not opened yet.
    #[error("IGeneralService is not opened on this NifmService")]
    NotOpened,
    /// IPC dispatch failed.
    #[error("failed to dispatch SetWirelessCommunicationEnabled")]
    Dispatch(#[source] DispatchError),
}

/// Error returned by [`NifmService::create_request`].
#[derive(Debug, thiserror::Error)]
pub enum CreateRequestSurfaceError {
    /// `IGeneralService` is not opened yet.
    #[error("IGeneralService is not opened on this NifmService")]
    NotOpened,
    /// `CreateRequest` IPC dispatch or sub-object retrieval failed.
    #[error("failed to dispatch CreateRequest")]
    Create(#[source] CreateRequestError),
    /// `CreateRequest` returned a zero sub-object id.
    #[error("CreateRequest response did not include a sub-object")]
    MissingObject,
    /// `GetSystemEventReadableHandles` failed; the freshly-created sub-object
    /// is closed via `Drop` when this error surfaces.
    #[error("failed to retrieve IRequest system event handles")]
    GetEvents(#[source] GetSystemEventHandlesError),
}

/// `IRequest` sub-object obtained via [`NifmService::create_request`].
///
/// The lifetime parameter ties the request to its parent service so the
/// underlying domain session outlives the sub-object. Dropping the request
/// fires a domain `Close` request for the sub-object via the inner
/// [`DomainObject`]'s `Drop`; the two event handles are released by their
/// own `Drop` implementations on [`EventHandle`].
pub struct NifmRequest<'svc> {
    /// Borrowed view onto the IRequest object inside the parent domain.
    object: DomainObject<'svc>,
    /// First event from `GetSystemEventReadableHandles`. Server-side autoclear.
    event_request_state: EventHandle,
    /// Second event from `GetSystemEventReadableHandles`.
    event1: EventHandle,
    /// libnx-parity cache: last `GetRequestState` value.
    cached_state: Cell<NifmRequestState>,
    /// libnx-parity cache: whether the last `GetResult` returned success.
    cached_res_ok: Cell<bool>,
}

impl NifmRequest<'_> {
    /// Returns a reference to the readable `event_request_state` handle
    /// (autoclear=true on the server side).
    #[inline]
    pub fn event_request_state(&self) -> &EventHandle {
        &self.event_request_state
    }

    /// Returns a reference to the secondary readable event handle.
    #[inline]
    pub fn event1(&self) -> &EventHandle {
        &self.event1
    }

    /// Returns the cached request state from the last fetch. Until any
    /// `GetRequestState` call succeeds this returns
    /// [`NifmRequestState::Unknown1`], matching libnx's initial value.
    #[inline]
    pub fn cached_state(&self) -> NifmRequestState {
        self.cached_state.get()
    }

    /// `Cancel` (cmd 3).
    pub fn cancel(&self) -> Result<(), DispatchError> {
        request::cancel(self.object.as_borrowed())
    }

    /// `Submit` (cmd 4). Raw single-shot variant; see [`Self::submit_libnx`]
    /// for the libnx-style wrapper that gates submission on the current state.
    pub fn submit(&self) -> Result<(), DispatchError> {
        request::submit(self.object.as_borrowed())
    }

    /// `GetRequestState` (cmd 0). Returns the raw `u32` the server reported.
    pub fn get_request_state_raw(&self) -> Result<u32, DispatchError> {
        request::get_request_state_raw(self.object.as_borrowed())
    }

    /// `GetResult` (cmd 1). The CMIF result code *is* the Switch-side `Result`.
    pub fn get_result_raw(&self) -> Result<(), DispatchError> {
        request::get_result(self.object.as_borrowed())
    }

    /// `SetNetworkProfileId` (cmd 9).
    pub fn set_network_profile_id(&self, uuid: Uuid) -> Result<(), DispatchError> {
        request::set_network_profile_id(self.object.as_borrowed(), uuid)
    }

    /// `GetAppletInfo` (cmd 21).
    pub fn get_applet_info(
        &self,
        theme_color: u32,
        buffer: &mut [u8],
    ) -> Result<AppletInfo, DispatchError> {
        request::get_applet_info(self.object.as_borrowed(), theme_color, buffer)
    }

    /// `SetKeptInSleep` (cmd 23, `[3.0.0+]`). Caller must guard on hosversion.
    pub fn set_kept_in_sleep(&self, flag: bool) -> Result<(), DispatchError> {
        request::set_kept_in_sleep(self.object.as_borrowed(), flag)
    }

    /// `RegisterSocketDescriptor` (cmd 24, `[3.0.0+]`).
    pub fn register_socket_descriptor(&self, sockfd: i32) -> Result<(), DispatchError> {
        request::register_socket_descriptor(self.object.as_borrowed(), sockfd)
    }

    /// `UnregisterSocketDescriptor` (cmd 25, `[3.0.0+]`).
    pub fn unregister_socket_descriptor(&self, sockfd: i32) -> Result<(), DispatchError> {
        request::unregister_socket_descriptor(self.object.as_borrowed(), sockfd)
    }

    /// Mirrors libnx's `nifmGetRequestState`:
    /// - If the `event_request_state` is not yet signaled (server-side
    ///   autoclear), return the cached state.
    /// - Otherwise refresh the cached state + result via cmds 0 and 1, and
    ///   return the refreshed state.
    pub fn get_request_state(&self) -> Result<NifmRequestState, DispatchError> {
        let signaled = wait_synchronization(&self.event_request_state, Some(Duration::ZERO));
        if signaled.is_err() {
            // Either timeout or another error: either way, the cached state is
            // returned without surfacing the wait error.
            return Ok(self.cached_state.get());
        }
        self.refresh_state();
        Ok(self.cached_state.get())
    }

    /// Mirrors libnx's `nifmGetResult`: returns the cached result if the event
    /// has not signaled, otherwise refreshes the cache and returns the new
    /// result.
    pub fn get_result(&self) -> Result<(), DispatchError> {
        let signaled = wait_synchronization(&self.event_request_state, Some(Duration::ZERO));
        if signaled.is_ok() {
            self.refresh_state();
        }
        if self.cached_res_ok.get() {
            Ok(())
        } else {
            // libnx returns the cached raw `Result`; without a typed error
            // value to thread through here, surface the most recent dispatch
            // outcome from `GetResult` directly.
            request::get_result(self.object.as_borrowed())
        }
    }

    /// Mirrors libnx's `nifmRequestSubmit`: re-reads the current state, and if
    /// it is in a state that accepts submission, fires cmd 4 and refreshes the
    /// cache. Dispatch errors from the submit itself are swallowed to match
    /// libnx (`sdknso ignores error`).
    pub fn submit_libnx(&self) -> Result<(), DispatchError> {
        let state = self.get_request_state()?;
        if matches!(
            state,
            NifmRequestState::Unknown1
                | NifmRequestState::OnHold
                | NifmRequestState::Available
                | NifmRequestState::Unknown5
        ) {
            let _ = request::submit(self.object.as_borrowed());
            self.refresh_state();
        }
        Ok(())
    }

    /// Submits the request and then polls the state until it is no longer
    /// `OnHold`, waiting on `event_request_state` between polls.
    pub fn submit_and_wait(&self) -> Result<(), DispatchError> {
        /// How long one poll iteration waits on the request-state event.
        const POLL_INTERVAL: Duration = Duration::from_secs(10);

        self.submit_libnx()?;
        loop {
            let state = self.get_request_state()?;
            if state != NifmRequestState::OnHold {
                return Ok(());
            }
            let waited = wait_synchronization(&self.event_request_state, Some(POLL_INTERVAL));
            if waited.is_ok() {
                self.refresh_state();
                return Ok(());
            }
        }
    }

    /// Mirrors libnx's `_nifmUpdateState`: refreshes the cached state and
    /// result via cmds 0 and 1.
    fn refresh_state(&self) {
        match request::get_request_state_raw(self.object.as_borrowed()) {
            Ok(raw) => self.cached_state.set(NifmRequestState::from_raw(raw)),
            // libnx zeroes the cache on dispatch failure.
            Err(_) => self.cached_state.set(NifmRequestState::Invalid),
        }
        self.cached_res_ok
            .set(request::get_result(self.object.as_borrowed()).is_ok());
    }
}

impl Drop for NifmRequest<'_> {
    fn drop(&mut self) {
        // Close the readable events. The IRequest sub-object itself is closed
        // by [`DomainObject`]'s own `Drop`. We bypass the typed close API
        // because it only takes session handles; the raw syscall is the right
        // tool.
        // SAFETY: both handles came from the kernel via `GetSystemEventReadableHandles`.
        unsafe {
            let _ = nx_svc::raw::close_handle(self.event_request_state.to_raw());
            let _ = nx_svc::raw::close_handle(self.event1.to_raw());
        }
    }
}
