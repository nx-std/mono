//! Account service (`acc:*`) implementation.
//!
//! Provides access to user account management on the Nintendo Switch,
//! including user enumeration, profile retrieval, and profile icon loading.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif_application`],
//! [`connect_cmif_system`], or [`connect_cmif_administrator`] obtain the root
//! session, then [`AccService::get_profile`] returns an [`AccountProfile`] with
//! its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `acc.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD` and selects the service name via
//! `AccountServiceType`. This crate follows the convention of the other
//! `nx-service-*` crates: connect once via one of the `connect_cmif_*`
//! functions, then call methods directly.
//!
//! libnx calls `InitializeApplicationInfo` automatically during
//! `accountInitialize` for the application variant. This crate exposes
//! [`AccService::initialize_application_info_legacy`] and
//! [`AccService::initialize_application_info`] as explicit methods per IC-4
//! (hosversion-unaware design) — the caller selects the correct variant.
//!
//! The `GetPreselectedUser` helper from libnx is not ported here because it
//! uses the applet launch parameter API, not the account service IPC.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, DispatchError, OwnedSessionHandle, Session},
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::GetProfileError,
    proto::{SERVICE_NAME_ADMINISTRATOR, SERVICE_NAME_APPLICATION, SERVICE_NAME_SYSTEM},
    types::{
        AccountNetworkServiceAccountId, AccountProfileBase, AccountUid, AccountUserData,
        USER_LIST_SIZE,
    },
};

/// Account root service wrapper.
///
/// Use [`get_profile`](Self::get_profile) to obtain a profile sub-object for
/// a specific user.
#[repr(transparent)]
pub struct AccService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for AccService {}
unsafe impl Sync for AccService {}

impl AccService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Gets the total number of user profiles.
    #[inline]
    pub fn get_user_count(&self) -> Result<i32, DispatchError> {
        cmif::get_user_count(&self.0)
    }

    /// Lists all user IDs.
    ///
    /// Fills the provided buffer with up to [`USER_LIST_SIZE`] user IDs.
    /// Invalid (all-zero) entries mark the end of the valid range.
    #[inline]
    pub fn list_all_users(
        &self,
        uids: &mut [AccountUid; USER_LIST_SIZE],
    ) -> Result<(), DispatchError> {
        cmif::list_all_users(&self.0, uids)
    }

    /// Gets the user ID for the last opened user.
    #[inline]
    pub fn get_last_opened_user(&self) -> Result<AccountUid, DispatchError> {
        cmif::get_last_opened_user(&self.0)
    }

    /// Gets a profile sub-object for the specified user.
    pub fn get_profile(&self, uid: AccountUid) -> Result<AccountProfile, GetProfileError> {
        let raw_handle = cmif::get_profile(&self.0, uid)?;

        // SAFETY: The server returned a freshly opened profile session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        let service = Session::new(handle, 0);

        Ok(AccountProfile(service))
    }

    /// Checks if user registration is permitted. Sends PID.
    #[inline]
    pub fn is_user_registration_request_permitted(&self) -> Result<bool, DispatchError> {
        cmif::is_user_registration_request_permitted(&self.0)
    }

    /// Selects a user without applet interaction.
    #[inline]
    pub fn try_select_user_without_interaction(
        &self,
        is_network_service_account_required: bool,
    ) -> Result<AccountUid, DispatchError> {
        cmif::try_select_user_without_interaction(&self.0, is_network_service_account_required)
    }

    /// Initializes application info (pre-6.0.0). Sends PID.
    ///
    /// Corresponds to command 100 in the acc service. Call this variant when
    /// targeting HOS versions before 6.0.0.
    #[inline]
    pub fn initialize_application_info_legacy(&self) -> Result<(), DispatchError> {
        cmif::initialize_application_info_legacy(&self.0)
    }

    /// Initializes application info (6.0.0+). Sends PID.
    ///
    /// Corresponds to command 140 in the acc service. Call this variant when
    /// targeting HOS 6.0.0 or later.
    #[inline]
    pub fn initialize_application_info(&self) -> Result<(), DispatchError> {
        cmif::initialize_application_info(&self.0)
    }
}

/// Account profile session wrapper.
///
/// Obtained via [`AccService::get_profile`]. Owns its own independent session
/// handle.
#[repr(transparent)]
pub struct AccountProfile(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for AccountProfile {}
unsafe impl Sync for AccountProfile {}

impl AccountProfile {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Gets profile base and user data.
    #[inline]
    pub fn get(&self, userdata: &mut AccountUserData) -> Result<AccountProfileBase, DispatchError> {
        cmif::profile_get(&self.0, userdata)
    }

    /// Gets profile base only, without user data.
    #[inline]
    pub fn get_base(&self) -> Result<AccountProfileBase, DispatchError> {
        cmif::profile_get_base(&self.0)
    }

    /// Gets the profile icon image size in bytes.
    #[inline]
    pub fn get_image_size(&self) -> Result<u32, DispatchError> {
        cmif::profile_get_image_size(&self.0)
    }

    /// Loads the JPEG profile icon image into the provided buffer.
    ///
    /// Returns the number of bytes actually written. The returned size matches
    /// what [`get_image_size`](Self::get_image_size) reports.
    #[inline]
    pub fn load_image(&self, buf: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::profile_load_image(&self.0, buf)
    }
}

/// Connects to the account application service (`acc:u0`) using CMIF.
pub fn connect_cmif_application(sm: &SmService) -> Result<AccService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_APPLICATION)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AccService(service))
}

/// Connects to the account system service (`acc:u1`) using CMIF.
pub fn connect_cmif_system(sm: &SmService) -> Result<AccService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_SYSTEM)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AccService(service))
}

/// Connects to the account administrator service (`acc:su`) using CMIF.
pub fn connect_cmif_administrator(sm: &SmService) -> Result<AccService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_ADMINISTRATOR)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(AccService(service))
}

/// Error returned by the `connect_cmif_*` functions.
#[derive(Debug, thiserror::Error)]
#[error("failed to get account service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
