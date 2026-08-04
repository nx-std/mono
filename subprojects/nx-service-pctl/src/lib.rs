//! Parental controls (`pctl`) service implementation.
//!
//! Provides access to the parental controls service for querying restriction
//! state, safety levels, and play-timer events on the Nintendo Switch.
//!
//! ## Architecture
//!
//! The service operates in domain mode. [`connect_cmif`] (or
//! [`connect_cmif_legacy`]) obtains the root `IParentalControlServiceFactory`
//! session, converts it to a domain, and creates an
//! `IParentalControlService` sub-object.
//!
//! ## Divergence from libnx
//!
//! libnx's `pctl.c` keeps a guarded global singleton with a cascading service
//! name lookup (`pctl:a` → `pctl:s` → `pctl:r` → `pctl`) and uses
//! hosversion to select between `CreateService` (cmd 0, pre-4.0.0) and
//! `CreateServiceWithoutInitialize` (cmd 1, 4.0.0+). This crate uses
//! `pctl:a` only and exposes paired [`connect_cmif_legacy`] /
//! [`connect_cmif`] functions per IC-4.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose the
//! appropriate connect function based on the target firmware version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    ConvertToDomainError,
    DispatchError,
    Domain,
    DomainObjectRef,
    Session,
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use nx_sf::service::DispatchError as IpcDispatchError;

pub use self::{
    cmif::{
        CreateServiceError,
        GetEventError,
    },
    proto::{
        SERVICE_NAME,
        SERVICE_NAME_A,
        SERVICE_NAME_R,
        SERVICE_NAME_S,
    },
    types::PctlRestrictionSettings,
};

/// Connected parental controls service wrapper.
///
/// The service operates in domain mode; the IParentalControlService sub-object
/// shares the same kernel session as the factory. Dropping the service closes
/// the underlying domain session, which cascades to the sub-object server-side.
pub struct PctlService {
    factory: Domain,
    object_id: u32,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PctlService {}
unsafe impl Sync for PctlService {}

impl PctlService {
    /// Checks whether parental controls restrictions are temporarily unlocked.
    #[inline]
    pub fn is_restriction_temporary_unlocked(&self) -> Result<bool, DispatchError> {
        cmif::is_restriction_temporary_unlocked(self.subobject()).map(|v| v & 1 != 0)
    }

    /// Confirms stereo vision (VR mode) permission. [4.0.0+]
    #[inline]
    pub fn confirm_stereo_vision_permission(&self) -> Result<(), DispatchError> {
        cmif::confirm_stereo_vision_permission(self.subobject())
    }

    /// Checks whether parental controls are enabled.
    #[inline]
    pub fn is_restriction_enabled(&self) -> Result<bool, DispatchError> {
        cmif::is_restriction_enabled(self.subobject()).map(|v| v & 1 != 0)
    }

    /// Gets the current safety level.
    #[inline]
    pub fn get_safety_level(&self) -> Result<u32, DispatchError> {
        cmif::get_safety_level(self.subobject())
    }

    /// Gets the current restriction settings.
    #[inline]
    pub fn get_current_settings(&self) -> Result<PctlRestrictionSettings, DispatchError> {
        cmif::get_current_settings(self.subobject())
    }

    /// Gets the count of applications that have free communication.
    #[inline]
    pub fn get_free_communication_application_list_count(&self) -> Result<u32, DispatchError> {
        cmif::get_free_communication_application_list_count(self.subobject())
    }

    /// Resets the confirmation done by
    /// [`confirm_stereo_vision_permission`](Self::confirm_stereo_vision_permission).
    /// [5.0.0+]
    #[inline]
    pub fn reset_confirmed_stereo_vision_permission(&self) -> Result<(), DispatchError> {
        cmif::reset_confirmed_stereo_vision_permission(self.subobject())
    }

    /// Checks whether stereo vision (VR mode) is permitted. [5.0.0+]
    #[inline]
    pub fn is_stereo_vision_permitted(&self) -> Result<bool, DispatchError> {
        cmif::is_stereo_vision_permitted(self.subobject()).map(|v| v & 1 != 0)
    }

    /// Checks whether pairing is active.
    #[inline]
    pub fn is_pairing_active(&self) -> Result<bool, DispatchError> {
        cmif::is_pairing_active(self.subobject()).map(|v| v & 1 != 0)
    }

    /// Gets the synchronization event handle.
    ///
    /// The caller is responsible for managing the handle lifetime.
    #[inline]
    pub fn get_synchronization_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(self.subobject(), proto::GET_SYNCHRONIZATION_EVENT)
    }

    /// Gets the play-timer event handle for requesting suspension.
    ///
    /// The caller is responsible for managing the handle lifetime.
    #[inline]
    pub fn get_play_timer_event_to_request_suspension(&self) -> Result<u32, GetEventError> {
        cmif::get_event(
            self.subobject(),
            proto::GET_PLAY_TIMER_EVENT_TO_REQUEST_SUSPENSION,
        )
    }

    /// Checks whether the play-timer alarm is disabled. [4.0.0+]
    #[inline]
    pub fn is_play_timer_alarm_disabled(&self) -> Result<bool, DispatchError> {
        cmif::is_play_timer_alarm_disabled(self.subobject()).map(|v| v & 1 != 0)
    }

    /// Gets the unlinked event handle.
    ///
    /// The caller is responsible for managing the handle lifetime.
    #[inline]
    pub fn get_unlinked_event(&self) -> Result<u32, GetEventError> {
        cmif::get_event(self.subobject(), proto::GET_UNLINKED_EVENT)
    }

    /// Addresses the IParentalControlService sub-object. The view closes
    /// nothing: the server-side object is released implicitly when the parent
    /// `Domain` is dropped.
    #[inline]
    fn subobject(&self) -> DomainObjectRef<'_> {
        // SAFETY: `object_id` was returned by `create_service*` on this same
        // factory domain, and the server-side object stays alive for the
        // lifetime of `self.factory`.
        DomainObjectRef::from_raw_unchecked(self.factory.as_borrowed(), self.object_id)
            .expect("PctlService holds a non-zero sub-object id")
    }
}

/// Connects to the parental controls service using CMIF (pre-4.0.0).
///
/// Uses `CreateService` (cmd 0) to obtain the IParentalControlService
/// sub-object. No post-initialization command is sent.
///
/// On 4.0.0+ use [`connect_cmif`].
pub fn connect_cmif_legacy(sm: &SmService) -> Result<PctlService, ConnectCmifError> {
    let factory = connect_factory(sm)?;

    let object_id = cmif::create_service_legacy(factory.as_borrowed())
        .map_err(ConnectCmifError::CreateService)?;

    Ok(PctlService { factory, object_id })
}

/// Connects to the parental controls service using CMIF (4.0.0+).
///
/// Uses `CreateServiceWithoutInitialize` (cmd 1) to obtain the
/// IParentalControlService sub-object, then calls
/// `ConfirmLaunchApplicationPermission` (cmd 1) on the sub-object as
/// post-initialization.
///
/// On pre-4.0.0 use [`connect_cmif_legacy`].
pub fn connect_cmif(sm: &SmService) -> Result<PctlService, ConnectCmifError> {
    let factory = connect_factory(sm)?;

    let object_id =
        cmif::create_service(factory.as_borrowed()).map_err(ConnectCmifError::CreateService)?;

    let pctl = PctlService { factory, object_id };

    cmif::confirm_launch_application_permission(pctl.subobject())
        .map_err(ConnectCmifError::PostInit)?;

    Ok(pctl)
}

/// Connects to the pctl:a factory and converts to domain mode.
fn connect_factory(sm: &SmService) -> Result<Domain, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME_A)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::open(handle);

    session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))
}

/// Errors returned by [`connect_cmif`] and [`connect_cmif_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `pctl:a` failed.
    #[error("failed to look up pctl:a service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on pctl:a session")]
    ConvertToDomain(#[source] ConvertToDomainError),
    /// Creating the IParentalControlService sub-object failed.
    #[error("failed to create IParentalControlService sub-object")]
    CreateService(#[source] CreateServiceError),
    /// Post-initialization command failed (4.0.0+ only).
    #[error("ConfirmLaunchApplicationPermission post-init failed")]
    PostInit(#[source] DispatchError),
}
