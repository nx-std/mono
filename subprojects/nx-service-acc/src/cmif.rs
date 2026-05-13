//! CMIF protocol operations for the account service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Service};

use crate::{
    dispatch::{dispatch_in_out, dispatch_out},
    proto,
    types::{
        AccountProfileBase, AccountUid, AccountUserData, InitializeApplicationInfoIn,
        IsUserRegistrationPermittedIn, USER_LIST_SIZE,
    },
};

/// Gets the total number of user profiles.
pub(crate) fn get_user_count(service: &Service) -> Result<i32, DispatchError> {
    dispatch_out(service, proto::GET_USER_COUNT)
}

/// Lists all user IDs via HipcPointer output buffer.
pub(crate) fn list_all_users(
    service: &Service,
    uids: &mut [AccountUid; USER_LIST_SIZE],
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::LIST_ALL_USERS)
        .buffer(
            uids.as_mut_ptr().cast::<u8>(),
            size_of::<[AccountUid; USER_LIST_SIZE]>(),
            BufferAttr::OUT.or(BufferAttr::HIPC_POINTER),
        )
        .send()?;
    Ok(())
}

/// Gets the last opened user ID.
pub(crate) fn get_last_opened_user(service: &Service) -> Result<AccountUid, DispatchError> {
    dispatch_out(service, proto::GET_LAST_OPENED_USER)
}

/// Gets an IProfile sub-object for a user. Returns the move handle.
pub(crate) fn get_profile(service: &Service, uid: AccountUid) -> Result<u32, GetProfileError> {
    // SAFETY: `uid` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::GET_PROFILE)
            .in_raw((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
            .send()
            .map_err(GetProfileError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(GetProfileError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Initializes application info (pre-6.0.0). Sends PID.
pub(crate) fn initialize_application_info_legacy(service: &Service) -> Result<(), DispatchError> {
    let input = InitializeApplicationInfoIn { pid_placeholder: 0 };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::INITIALIZE_APPLICATION_INFO_LEGACY)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<InitializeApplicationInfoIn>(),
            )
            .send_pid()
            .send()?;
    }
    Ok(())
}

/// Initializes application info (6.0.0+). Sends PID.
pub(crate) fn initialize_application_info(service: &Service) -> Result<(), DispatchError> {
    let input = InitializeApplicationInfoIn { pid_placeholder: 0 };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::INITIALIZE_APPLICATION_INFO)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<InitializeApplicationInfoIn>(),
            )
            .send_pid()
            .send()?;
    }
    Ok(())
}

/// Checks if user registration is permitted. Sends PID.
pub(crate) fn is_user_registration_request_permitted(
    service: &Service,
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
    service: &Service,
    is_network_service_account_required: bool,
) -> Result<AccountUid, DispatchError> {
    let input: u8 = u8::from(is_network_service_account_required);
    dispatch_in_out(service, proto::TRY_SELECT_USER_WITHOUT_INTERACTION, input)
}

// ---------------------------------------------------------------------------
// IProfile commands
// ---------------------------------------------------------------------------

/// Gets profile data (base + optional user data).
pub(crate) fn profile_get(
    service: &Service,
    userdata: &mut AccountUserData,
) -> Result<AccountProfileBase, DispatchError> {
    let result = service
        .dispatch(proto::PROFILE_GET)
        .out_size(size_of::<AccountProfileBase>())
        .buffer(
            (userdata as *mut AccountUserData).cast::<u8>(),
            size_of::<AccountUserData>(),
            BufferAttr::OUT
                .or(BufferAttr::FIXED_SIZE)
                .or(BufferAttr::HIPC_POINTER),
        )
        .send()?;

    // SAFETY: the response payload is at least `size_of::<AccountProfileBase>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<AccountProfileBase>()) })
}

/// Gets profile base only (no user data buffer).
pub(crate) fn profile_get_base(service: &Service) -> Result<AccountProfileBase, DispatchError> {
    dispatch_out(service, proto::PROFILE_GET_BASE)
}

/// Gets the profile icon image size.
pub(crate) fn profile_get_image_size(service: &Service) -> Result<u32, DispatchError> {
    dispatch_out(service, proto::PROFILE_GET_IMAGE_SIZE)
}

/// Loads the JPEG profile icon image. Returns bytes written.
pub(crate) fn profile_load_image(service: &Service, buf: &mut [u8]) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::PROFILE_LOAD_IMAGE)
        .out_size(size_of::<u32>())
        .buffer(
            buf.as_mut_ptr(),
            buf.len(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

/// CMIF request with a single `Copy` input, a single `Copy` output, and PID.
#[inline]
fn dispatch_in_out_with_pid<I: Copy, O: Copy>(
    service: &Service,
    cmd_id: u32,
    input: I,
) -> Result<O, DispatchError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<I>())
            .out_size(size_of::<O>())
            .send_pid()
            .send()?
    };
    // SAFETY: the response payload is at least `size_of::<O>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
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
