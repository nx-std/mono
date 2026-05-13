//! CMIF protocol operations for the friends service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject};

use crate::{
    proto,
    types::{AccountUid, FriendsUserSetting},
};

/// Creates an IFriendService sub-object via domain dispatch (cmd 0).
///
/// Returns the raw sub-object ID for the new `IFriendService` domain object.
pub(crate) fn create_friend_service(domain: &Domain) -> Result<u32, CreateFriendServiceError> {
    let result = domain
        .dispatch(proto::CREATE_FRIEND_SERVICE)
        .out_objects(1)
        .send()
        .map_err(CreateFriendServiceError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(CreateFriendServiceError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Gets the user setting for the given account UID (cmd 20800).
///
/// The output is written into `out` via a fixed-size HipcPointer buffer.
pub(crate) fn get_user_setting(
    object: &DomainObject<'_>,
    uid: AccountUid,
    out: &mut FriendsUserSetting,
) -> Result<(), DispatchError> {
    // SAFETY: `uid` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::GET_USER_SETTING)
            .in_raw((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
            .buffer(
                (out as *mut FriendsUserSetting).cast::<u8>(),
                size_of::<FriendsUserSetting>(),
                BufferAttr::OUT
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .send()
            .map(|_| ())
    }
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
