//! Local Play Peer-to-Peer (`lp2p`) service implementation.
//!
//! Provides access to the LP2P service for local-WLAN communications with
//! accessories. Only available on \[9.1.0+\].
//!
//! ## Architecture
//!
//! - **Root session** (`lp2p:app` or `lp2p:sys`): Converted to domain mode
//!   with a session pool (4 slots by default) for concurrent IPC dispatch.
//!
//! - **INetworkService** sub-object: Domain child created via cmd 0 on the
//!   root session. Provides scanning, group management, and data transfer
//!   commands. Dispatched through the session pool.
//!
//! - **INetworkServiceMonitor** sub-object: Separate non-domain session
//!   obtained from a second SM connection (cmd 8). Provides state queries,
//!   event attachment, and join/leave operations. Dispatched directly.
//!
//! ## Divergence from libnx
//!
//! libnx's `lp2p.c` keeps guarded global singletons managed by
//! `NX_GENERATE_SERVICE_GUARD`, with a `SessionMgr` for concurrent domain
//! dispatch. This crate follows the convention of the other `nx-service-*`
//! crates: connect once via [`connect_cmif`], then call methods directly.

#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

use alloc::{boxed::Box, vec::Vec};

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, Session, clone_current_object};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod session;
pub mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

use crate::session::SessionPool;
pub use crate::{
    cmif::AttachEventError,
    proto::{SERVICE_NAME_APP, SERVICE_NAME_SYS},
    types::{
        AdvertiseDataResult, Lp2pGroupId, Lp2pGroupInfo, Lp2pIpConfig, Lp2pMacAddress,
        Lp2pNodeInfo, Lp2pScanResult, RecvFromOtherGroupResult,
    },
};

/// Default number of session pool slots (matches libnx's `sessionmgrCreate(..., 0x4)`).
pub const DEFAULT_SESSION_COUNT: usize = 4;

/// Service type selector for LP2P.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lp2pServiceType {
    /// Application service (`lp2p:app`).
    App,
    /// System service (`lp2p:sys`).
    System,
}

/// Connected LP2P service wrapper.
///
/// Holds both the domain-mode `INetworkService` (dispatched through a session
/// pool) and the non-domain `INetworkServiceMonitor`. Dropping the service
/// closes all pool sessions and the monitor session.
pub struct Lp2pService {
    pool: SessionPool,
    network_service_object_id: u32,
    monitor: Session,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `nx_std_sync::Mutex` / `Condvar` based pool. The HOS kernel serializes
// `svcSendSyncRequest` per session handle; concurrent IPC calls acquire
// distinct pool slots for domain dispatch. The monitor session is only
// accessed through `&self` methods that serialize at the Rust borrow level.
unsafe impl Send for Lp2pService {}
unsafe impl Sync for Lp2pService {}

impl Lp2pService {
    /// Returns the `INetworkService` interface for group management and
    /// data transfer commands.
    pub fn network_service(&self) -> Lp2pNetworkService<'_> {
        Lp2pNetworkService { service: self }
    }

    /// Returns the `INetworkServiceMonitor` interface for state queries,
    /// event attachment, and join/leave operations.
    pub fn network_service_monitor(&self) -> Lp2pNetworkServiceMonitor<'_> {
        Lp2pNetworkServiceMonitor { service: self }
    }
}

/// INetworkService interface (domain, dispatched through session pool).
pub struct Lp2pNetworkService<'svc> {
    service: &'svc Lp2pService,
}

impl Lp2pNetworkService<'_> {
    /// Scans for nearby groups (cmd 512).
    ///
    /// Returns the number of results written to `results`.
    pub fn scan(
        &self,
        info: &Lp2pGroupInfo,
        results: &mut [Lp2pScanResult],
    ) -> Result<i32, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::scan(&object, info, results)
    }

    /// Creates a group (cmd 768).
    pub fn create_group(&self, info: &Lp2pGroupInfo) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::create_group(&object, info)
    }

    /// Destroys the previously created group (cmd 776).
    pub fn destroy_group(&self) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::destroy_group(&object)
    }

    /// Sets the advertise data for the current group (cmd 784).
    pub fn set_advertise_data(&self, data: &[u8]) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::set_advertise_data(&object, data)
    }

    /// Sends data to another group (cmd 1536).
    #[allow(clippy::too_many_arguments)]
    pub fn send_to_other_group(
        &self,
        data: &[u8],
        addr: Lp2pMacAddress,
        group_id: Lp2pGroupId,
        frequency: i16,
        channel: i16,
        flags: u32,
    ) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::send_to_other_group(&object, data, addr, group_id, frequency, channel, flags)
    }

    /// Receives data from another group (cmd 1544).
    pub fn recv_from_other_group(
        &self,
        flags: u32,
        buffer: &mut [u8],
    ) -> Result<RecvFromOtherGroupResult, DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        let out = cmif::recv_from_other_group(&object, flags, buffer)?;
        Ok(RecvFromOtherGroupResult {
            addr: out.addr,
            unk0: out.unk0,
            unk1: out.unk1 as i32,
            out_size: out.out_size as u64,
            unk2: out.unk2,
        })
    }

    /// Adds an acceptable group ID for receiving inter-group data (cmd 1552).
    pub fn add_acceptable_group_id(&self, group_id: Lp2pGroupId) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::add_acceptable_group_id(&object, group_id)
    }

    /// Removes the acceptable group ID (cmd 1560).
    pub fn remove_acceptable_group_id(&self) -> Result<(), DispatchError> {
        let guard = self.service.pool.acquire();
        // SAFETY: `network_service_object_id` was returned by the server and
        // validated at `connect_cmif`; the pool guard makes this slot exclusive.
        let object = unsafe { guard.open_object_raw(self.service.network_service_object_id) }
            .expect("network_service object id validated at connect_cmif");
        cmif::remove_acceptable_group_id(&object)
    }
}

/// INetworkServiceMonitor interface (non-domain, direct dispatch).
pub struct Lp2pNetworkServiceMonitor<'svc> {
    service: &'svc Lp2pService,
}

impl Lp2pNetworkServiceMonitor<'_> {
    /// Attaches the network interface state change event (cmd 256).
    ///
    /// Returns the event handle (autoclear=false).
    pub fn attach_network_interface_state_change_event(&self) -> Result<u32, AttachEventError> {
        cmif::attach_network_interface_state_change_event(&self.service.monitor)
    }

    /// Gets the last network interface error (cmd 264).
    ///
    /// Returns `Ok(())` if no error, or the service error as a `DispatchError`.
    pub fn get_network_interface_last_error(&self) -> Result<(), DispatchError> {
        cmif::get_network_interface_last_error(&self.service.monitor)
    }

    /// Gets the current role (cmd 272).
    pub fn get_role(&self) -> Result<u8, DispatchError> {
        cmif::get_role(&self.service.monitor)
    }

    /// Gets advertise data with role validation (cmd 280).
    pub fn get_advertise_data(
        &self,
        buffer: &mut [u8],
    ) -> Result<AdvertiseDataResult, DispatchError> {
        let out =
            cmif::get_advertise_data(&self.service.monitor, proto::GET_ADVERTISE_DATA, buffer)?;
        Ok(AdvertiseDataResult {
            transfer_size: out.transfer_size,
            original_size: out.original_size,
        })
    }

    /// Gets advertise data without role validation (cmd 281).
    pub fn get_advertise_data_2(
        &self,
        buffer: &mut [u8],
    ) -> Result<AdvertiseDataResult, DispatchError> {
        let out =
            cmif::get_advertise_data(&self.service.monitor, proto::GET_ADVERTISE_DATA_2, buffer)?;
        Ok(AdvertiseDataResult {
            transfer_size: out.transfer_size,
            original_size: out.original_size,
        })
    }

    /// Gets the current group info (cmd 288).
    pub fn get_group_info(&self, out: &mut Lp2pGroupInfo) -> Result<(), DispatchError> {
        cmif::get_group_info(&self.service.monitor, out)
    }

    /// Joins a group (cmd 296).
    ///
    /// Writes the resulting group info into `out`.
    pub fn join(&self, out: &mut Lp2pGroupInfo, info: &Lp2pGroupInfo) -> Result<(), DispatchError> {
        cmif::join(&self.service.monitor, out, info)
    }

    /// Gets the group owner info (cmd 304).
    pub fn get_group_owner(&self) -> Result<Lp2pNodeInfo, DispatchError> {
        cmif::get_group_owner(&self.service.monitor)
    }

    /// Gets the IP configuration (cmd 312).
    pub fn get_ip_config(&self, out: &mut Lp2pIpConfig) -> Result<(), DispatchError> {
        cmif::get_ip_config(&self.service.monitor, out)
    }

    /// Leaves the current group (cmd 320).
    pub fn leave(&self) -> Result<u32, DispatchError> {
        cmif::leave(&self.service.monitor)
    }

    /// Attaches the join event (cmd 328).
    ///
    /// Returns the event handle (autoclear=false).
    pub fn attach_join_event(&self) -> Result<u32, AttachEventError> {
        cmif::attach_join_event(&self.service.monitor)
    }

    /// Gets the current group members (cmd 336).
    ///
    /// Returns the number of members written to `members`.
    pub fn get_members(&self, members: &mut [Lp2pNodeInfo]) -> Result<i32, DispatchError> {
        cmif::get_members(&self.service.monitor, members)
    }
}

/// Connects to the LP2P service using CMIF.
///
/// Sets up the domain-mode root session with a session pool, creates the
/// `INetworkService` sub-object (cmd 0), and creates the
/// `INetworkServiceMonitor` sub-object (cmd 8) on a separate non-domain
/// session.
///
/// `num_sessions` controls the pool size for concurrent domain dispatch
/// (libnx default is 4).
pub fn connect_cmif(
    sm: &SmService,
    service_type: Lp2pServiceType,
    num_sessions: usize,
) -> Result<Lp2pService, ConnectCmifError> {
    let service_name = match service_type {
        Lp2pServiceType::App => SERVICE_NAME_APP,
        Lp2pServiceType::System => SERVICE_NAME_SYS,
    };

    // Open the root session and convert to domain.
    let root_handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectCmifError::GetService)?;

    let root_session = Session::new(root_handle);
    let pointer_buffer_size = root_session.pointer_buffer_size();

    let root = root_session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    // Create INetworkService sub-object (cmd 0, domain child).
    let network_service_object_id =
        cmif::create_network_service(&root, 0x1).map_err(ConnectCmifError::CreateNetworkService)?;

    // Build session pool from cloned domain sessions. The first slot owns the
    // root domain handle; the remaining slots are cloned domain handles that
    // share the same server-side object table.
    let mut sessions: Vec<Domain> = Vec::with_capacity(num_sessions);
    sessions.push(root);
    for _ in 1..num_sessions {
        // SAFETY: cloning a domain session yields another kernel handle that
        // addresses the same domain object table on the server side.
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        let cloned_domain = unsafe {
            nx_sf::service::Domain::from_handle_unchecked(cloned_handle, pointer_buffer_size)
        };
        sessions.push(cloned_domain);
    }

    let pool = SessionPool::new(sessions.into_boxed_slice() as Box<[Domain]>);

    // Open a separate non-domain session for the monitor sub-object.
    let monitor_root_handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectCmifError::GetService)?;

    let monitor_root = Session::from_handle(monitor_root_handle, 0);

    // Create INetworkServiceMonitor sub-object (cmd 8, non-domain).
    // Returns a move handle — the sub-object owns its own session.
    let monitor_session_handle = cmif::create_network_service_monitor(&monitor_root)
        .map_err(ConnectCmifError::CreateMonitor)?;

    // Drop the temporary root session; the monitor sub-object has its own.
    drop(monitor_root);

    // SAFETY: the move handle from CreateNetworkServiceMonitor is a valid session.
    let monitor = Session::from_handle(
        unsafe { SessionHandle::from_raw(monitor_session_handle) },
        0,
    );

    Ok(Lp2pService {
        pool,
        network_service_object_id,
        monitor,
    })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `lp2p:app` or `lp2p:sys` failed.
    #[error("failed to look up lp2p service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the root session to a domain failed.
    #[error("failed to ConvertToDomain on lp2p root session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Creating the INetworkService sub-object failed.
    #[error("failed to create INetworkService sub-object")]
    CreateNetworkService(#[source] cmif::CreateNetworkServiceError),
    /// Cloning the root session for the pool failed.
    #[error("failed to clone lp2p session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
    /// Creating the INetworkServiceMonitor sub-object failed.
    #[error("failed to create INetworkServiceMonitor sub-object")]
    CreateMonitor(#[source] cmif::CreateMonitorError),
}
