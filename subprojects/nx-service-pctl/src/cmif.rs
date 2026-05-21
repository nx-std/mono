//! CMIF protocol operations for the parental controls service.

use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{DispatchError, Domain, DomainObject, OutHandleAttr};

use crate::{
    dispatch::{dispatch_no_io, dispatch_out},
    proto,
    types::PctlRestrictionSettings,
};

/// Creates an IParentalControlService sub-object (pre-4.0.0 wire format).
///
/// Sends PID. Returns the raw sub-object id; the underlying [`DomainObject`]
/// is kept alive via [`ManuallyDrop`] so the factory can re-open it per call.
pub(crate) fn create_service_legacy(domain: &Domain) -> Result<u32, CreateServiceError> {
    create_service_at(domain, proto::CREATE_SERVICE_LEGACY)
}

/// Creates an IParentalControlService sub-object (4.0.0+ wire format).
///
/// Sends PID. Returns the raw sub-object id; the underlying [`DomainObject`]
/// is kept alive via [`ManuallyDrop`] so the factory can re-open it per call.
pub(crate) fn create_service(domain: &Domain) -> Result<u32, CreateServiceError> {
    create_service_at(domain, proto::CREATE_SERVICE)
}

fn create_service_at(domain: &Domain, cmd_id: u32) -> Result<u32, CreateServiceError> {
    let pid_reserved: u64 = 0;
    // SAFETY: `pid_reserved` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const pid_reserved).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let mut result = domain
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateServiceError::MissingObject)?;
    Ok(ManuallyDrop::new(object).object_id().to_raw())
}

/// Confirms launch-application permission (post-init on 4.0.0+).
pub(crate) fn confirm_launch_application_permission(
    object: &DomainObject<'_>,
) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONFIRM_LAUNCH_APPLICATION_PERMISSION)
}

/// Checks whether restrictions are temporarily unlocked.
pub(crate) fn is_restriction_temporary_unlocked(
    object: &DomainObject<'_>,
) -> Result<u8, DispatchError> {
    dispatch_out(object, proto::IS_RESTRICTION_TEMPORARY_UNLOCKED)
}

/// Confirms stereo vision permission. [4.0.0+]
pub(crate) fn confirm_stereo_vision_permission(
    object: &DomainObject<'_>,
) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONFIRM_STEREO_VISION_PERMISSION)
}

/// Checks whether restrictions are enabled.
pub(crate) fn is_restriction_enabled(object: &DomainObject<'_>) -> Result<u8, DispatchError> {
    dispatch_out(object, proto::IS_RESTRICTION_ENABLED)
}

/// Gets the current safety level.
pub(crate) fn get_safety_level(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::GET_SAFETY_LEVEL)
}

/// Gets the current restriction settings.
pub(crate) fn get_current_settings(
    object: &DomainObject<'_>,
) -> Result<PctlRestrictionSettings, DispatchError> {
    dispatch_out(object, proto::GET_CURRENT_SETTINGS)
}

/// Gets the count of free-communication applications.
pub(crate) fn get_free_communication_application_list_count(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::GET_FREE_COMMUNICATION_APPLICATION_LIST_COUNT)
}

/// Resets the stereo vision permission confirmation. [5.0.0+]
pub(crate) fn reset_confirmed_stereo_vision_permission(
    object: &DomainObject<'_>,
) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::RESET_CONFIRMED_STEREO_VISION_PERMISSION)
}

/// Checks whether stereo vision is permitted. [5.0.0+]
pub(crate) fn is_stereo_vision_permitted(object: &DomainObject<'_>) -> Result<u8, DispatchError> {
    dispatch_out(object, proto::IS_STEREO_VISION_PERMITTED)
}

/// Checks whether pairing is active.
pub(crate) fn is_pairing_active(object: &DomainObject<'_>) -> Result<u8, DispatchError> {
    dispatch_out(object, proto::IS_PAIRING_ACTIVE)
}

/// Gets an event copy handle for the given command.
pub(crate) fn get_event(object: &DomainObject<'_>, cmd_id: u32) -> Result<u32, GetEventError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(GetEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetEventError::MissingHandle);
    }
    Ok(result.copy_handles[0])
}

/// Checks whether the play-timer alarm is disabled. [4.0.0+]
pub(crate) fn is_play_timer_alarm_disabled(object: &DomainObject<'_>) -> Result<u8, DispatchError> {
    dispatch_out(object, proto::IS_PLAY_TIMER_ALARM_DISABLED)
}

/// Error returned by [`create_service_legacy`] and [`create_service`].
#[derive(Debug, thiserror::Error)]
pub enum CreateServiceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateService")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain sub-object.
    #[error("CreateService response did not include the expected sub-object")]
    MissingObject,
}

/// Error returned by [`get_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetEventError {
    /// IPC dispatch failed.
    #[error("failed to dispatch event request")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected event copy handle.
    #[error("event request response did not include the expected handle")]
    MissingHandle,
}
