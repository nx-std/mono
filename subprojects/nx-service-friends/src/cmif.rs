//! CMIF protocol operations for the friends service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainRef,
};

use crate::{
    proto,
    types::{
        AccountUid,
        FriendsUserSetting,
    },
};

/// Creates an IFriendService sub-object via domain dispatch (cmd 0).
///
/// Returns the raw sub-object ID for the new `IFriendService` domain object.
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_friend_service(
    domain: DomainRef<'_>,
) -> Result<u32, CreateFriendServiceError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::CREATE_FRIEND_SERVICE)
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateFriendServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateFriendServiceError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Gets the user setting for the given account UID (cmd 20800).
///
/// The output is written into `out` via a fixed-size HipcPointer buffer.
pub(crate) fn get_user_setting(
    object: DomainObjectRef<'_>,
    uid: AccountUid,
    out: &mut FriendsUserSetting,
) -> Result<(), DispatchError> {
    // SAFETY: `uid` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<AccountUid>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
    };
    // SAFETY: `out` is a valid `&mut FriendsUserSetting`; viewing its bytes as
    // a mutable byte slice for the OUT pointer buffer is sound, and the byte
    // slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut FriendsUserSetting).cast::<u8>(),
            size_of::<FriendsUserSetting>(),
        )
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::GET_USER_SETTING)
        .in_raw(in_bytes)
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// Error returned by [`create_friend_service`].
#[derive(Debug, thiserror::Error)]
pub enum CreateFriendServiceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateFriendService")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain sub-object.
    #[error("CreateFriendService response did not include the expected sub-object")]
    MissingObject,
}
