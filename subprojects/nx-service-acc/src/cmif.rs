//! CMIF protocol operations for the account service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in_out,
        dispatch_out,
    },
    proto,
    types::{
        AccountProfileBase,
        AccountUid,
        AccountUserData,
        InitializeApplicationInfoIn,
        IsUserRegistrationPermittedIn,
        USER_LIST_SIZE,
    },
};

/// Gets the total number of user profiles.
pub(crate) fn get_user_count(service: &Session) -> Result<i32, DispatchError> {
    dispatch_out(service, proto::GET_USER_COUNT)
}

/// Lists all user IDs via HipcPointer output buffer.
pub(crate) fn list_all_users(
    service: &Session,
    uids: &mut [AccountUid; USER_LIST_SIZE],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::LIST_ALL_USERS)
        .out_buffer(uids.as_mut_bytes(), BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Gets the last opened user ID.
pub(crate) fn get_last_opened_user(service: &Session) -> Result<AccountUid, DispatchError> {
    dispatch_out(service, proto::GET_LAST_OPENED_USER)
}

/// Gets an IProfile sub-object for a user. Returns the move handle.
pub(crate) fn get_profile(service: &Session, uid: AccountUid) -> Result<u32, GetProfileError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_PROFILE)
        .in_raw(uid.as_bytes())
        .send(&mut ipc_buf)
        .map_err(GetProfileError::Dispatch)?;

    let Some(&handle) = result.move_handles.first() else {
        return Err(GetProfileError::MissingHandle);
    };
    Ok(handle)
}

/// Initializes application info (pre-6.0.0). Sends PID.
pub(crate) fn initialize_application_info_legacy(service: &Session) -> Result<(), DispatchError> {
    let input = InitializeApplicationInfoIn { pid_placeholder: 0 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::INITIALIZE_APPLICATION_INFO_LEGACY)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Initializes application info (6.0.0+). Sends PID.
pub(crate) fn initialize_application_info(service: &Session) -> Result<(), DispatchError> {
    let input = InitializeApplicationInfoIn { pid_placeholder: 0 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::INITIALIZE_APPLICATION_INFO)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)?;
    Ok(())
}

/// Checks if user registration is permitted. Sends PID.
pub(crate) fn is_user_registration_request_permitted(
    service: &Session,
) -> Result<bool, DispatchError> {
    let input = IsUserRegistrationPermittedIn { pid_placeholder: 0 };
    let raw: u8 = dispatch_in_out_with_pid(
        service,
        proto::IS_USER_REGISTRATION_REQUEST_PERMITTED,
        input,
    )?;
    Ok(raw & 1 != 0)
}

/// Selects a user without applet interaction.
pub(crate) fn try_select_user_without_interaction(
    service: &Session,
    is_network_service_account_required: bool,
) -> Result<AccountUid, DispatchError> {
    let input: u8 = u8::from(is_network_service_account_required);
    dispatch_in_out(service, proto::TRY_SELECT_USER_WITHOUT_INTERACTION, input)
}

/// Gets profile data (base + optional user data).
pub(crate) fn profile_get(
    service: &Session,
    userdata: &mut AccountUserData,
) -> Result<AccountProfileBase, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::PROFILE_GET)
        .out_size(size_of::<AccountProfileBase>())
        .out_buffer(
            userdata.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)?;

    Ok(*result.value::<AccountProfileBase>())
}

/// Gets profile base only (no user data buffer).
pub(crate) fn profile_get_base(service: &Session) -> Result<AccountProfileBase, DispatchError> {
    dispatch_out(service, proto::PROFILE_GET_BASE)
}

/// Gets the profile icon image size.
pub(crate) fn profile_get_image_size(service: &Session) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::PROFILE_GET_IMAGE_SIZE)
}

/// Loads the JPEG profile icon image. Returns bytes written.
pub(crate) fn profile_load_image(service: &Session, buf: &mut [u8]) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::PROFILE_LOAD_IMAGE)
        .out_size(size_of::<u32>())
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// CMIF request with a single `Copy` input, a single `Copy` output, and PID.
#[inline]
fn dispatch_in_out_with_pid<I, O>(
    service: &Session,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send_pid()
        .send(&mut ipc_buf)?;
    Ok(*result.value::<O>())
}

/// Error returned by [`get_profile`].
#[derive(Debug, thiserror::Error)]
pub enum GetProfileError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GetProfile")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("GetProfile response missing move handle")]
    MissingHandle,
}
