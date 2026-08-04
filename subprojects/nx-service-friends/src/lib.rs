//! Friends (`friend:*`) service implementation.
//!
//! Provides access to the friends service for querying user settings
//! and related friend-list data on the Nintendo Switch.
//!
//! ## Architecture
//!
//! The service operates in domain mode with a session pool (5 slots,
//! matching libnx's `sessionmgrCreate(..., 0x5)`). [`connect_cmif`] obtains
//! the root `IServiceCreator` session, converts it to a domain, clones the
//! session for the pool, and creates the `IFriendService` sub-object.
//!
//! ## Divergence from libnx
//!
//! libnx's `friends.c` keeps a guarded global singleton with a
//! `FriendsServiceType` enum that selects among `friend:u`, `friend:v`,
//! `friend:m`, `friend:s`, and `friend:a`. This crate exposes the same
//! variants via [`FriendsServiceType`] and lets the caller choose which
//! to connect to.
//!
//! libnx also includes `friendsTryPopFriendInvitationNotificationInfo` and
//! `friendsGetFriendInvitationNotificationEvent`, which are wrappers around
//! applet storage functions — NOT IPC calls to the friends service. Those
//! belong in a future `nx-service-applet` crate and are intentionally
//! omitted here.

#![no_std]

extern crate alloc;
extern crate nx_panic_handler as _; // provides #[panic_handler]

use alloc::{
    boxed::Box,
    vec::Vec,
};

use nx_service_sm::SmService;
use nx_sf::service::{
    ConvertToDomainError,
    DispatchError,
    Domain,
    Session,
    clone_current_object,
};

mod cmif;
mod proto;
mod session;
pub mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

use crate::{
    cmif::CreateFriendServiceError,
    session::{
        FRIENDS_POOL_SIZE,
        SessionPool,
    },
};
pub use crate::{
    proto::{
        SERVICE_NAME_ADMIN,
        SERVICE_NAME_MANAGER,
        SERVICE_NAME_SYSTEM,
        SERVICE_NAME_USER,
        SERVICE_NAME_VIEWER,
    },
    types::{
        AccountUid,
        FriendInvitationGameModeDescription,
        FriendInvitationGroupId,
        FriendInvitationId,
        FriendsUserSetting,
        InAppScreenName,
    },
};

/// Which `friend:*` service variant to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendsServiceType {
    /// `friend:u` — user access.
    User,
    /// `friend:v` — viewer access.
    Viewer,
    /// `friend:m` — manager access.
    Manager,
    /// `friend:s` — system access.
    System,
    /// `friend:a` — administrator access.
    Administrator,
}

impl FriendsServiceType {
    fn service_name(self) -> nx_sf::ServiceName {
        match self {
            Self::User => proto::SERVICE_NAME_USER,
            Self::Viewer => proto::SERVICE_NAME_VIEWER,
            Self::Manager => proto::SERVICE_NAME_MANAGER,
            Self::System => proto::SERVICE_NAME_SYSTEM,
            Self::Administrator => proto::SERVICE_NAME_ADMIN,
        }
    }
}

/// Connected friends service wrapper.
///
/// Operates in domain mode with a session pool for concurrent IPC dispatch.
/// The `IFriendService` sub-object shares the domain with the factory.
/// Dropping the service closes all pool sessions.
pub struct FriendsService {
    pool: SessionPool,
    friend_service_object_id: u32,
}

// SAFETY: every field is either an immutable kernel handle wrapper or a
// `nx_std_sync::Mutex` / `Condvar` based pool. Concurrent IPC calls from
// different threads acquire distinct pool slots.
unsafe impl Send for FriendsService {}
unsafe impl Sync for FriendsService {}

impl FriendsService {
    /// Gets the user setting for the given account UID.
    pub fn get_user_setting(
        &self,
        uid: AccountUid,
        out: &mut FriendsUserSetting,
    ) -> Result<(), DispatchError> {
        let guard = self.pool.acquire();
        // SAFETY: `friend_service_object_id` was validated at `connect_cmif`
        // and the kernel-side object stays alive for the lifetime of the
        // pool's domain sessions. The pool guard makes this slot exclusive,
        // so no other `DomainObject` in this `Domain` addresses the id.
        let object = guard
            .open_object_unchecked(self.friend_service_object_id)
            .expect("friend_service object id validated at connect_cmif");
        cmif::get_user_setting(object, uid, out)
    }
}

/// Connects to the friends service using CMIF.
///
/// Sets up domain conversion, the 5-session pool, and creates the
/// `IFriendService` sub-object (cmd 0).
pub fn connect_cmif(
    sm: &SmService,
    service_type: FriendsServiceType,
) -> Result<FriendsService, ConnectCmifError> {
    let creator_handle = sm
        .get_service_handle_cmif(service_type.service_name())
        .map_err(ConnectCmifError::GetService)?;

    let creator_session = Session::open(creator_handle);
    let pointer_buffer_size = creator_session.pointer_buffer_size();

    let creator = creator_session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    // Build session pool from cloned domain sessions. The first slot owns the
    // root domain handle; the remaining slots are cloned domain handles that
    // share the same server-side object table.
    let mut sessions: Vec<Domain> = Vec::with_capacity(FRIENDS_POOL_SIZE);
    sessions.push(creator);
    for _ in 1..FRIENDS_POOL_SIZE {
        let cloned_handle =
            clone_current_object(sessions[0].handle()).map_err(ConnectCmifError::CloneSession)?;
        // SAFETY: Cloning a domain session yields another kernel handle addressing the same
        // domain object table on the server side.
        let cloned_domain =
            nx_sf::service::Domain::new_unchecked(cloned_handle, pointer_buffer_size);
        sessions.push(cloned_domain);
    }

    let factory = &sessions[0];
    let friend_service_object_id = cmif::create_friend_service(factory.as_borrowed())
        .map_err(ConnectCmifError::CreateService)?;

    let pool = SessionPool::new(sessions.into_boxed_slice() as Box<[Domain]>);

    Ok(FriendsService {
        pool,
        friend_service_object_id,
    })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for the requested `friend:*` service failed.
    #[error("failed to look up friend:* service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on friend:* session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Cloning the creator session for the pool failed.
    #[error("failed to clone friend:* session for the pool")]
    CloneSession(#[source] nx_sf::service::CloneObjectError),
    /// Creating the IFriendService sub-object failed.
    #[error("failed to create IFriendService sub-object")]
    CreateService(#[source] CreateFriendServiceError),
}
