//! `nifm:u` / `nifm:s` / `nifm:a` static-service commands — used to spawn the
//! `IGeneralService` sub-object on the converted-to-domain creator session.

use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{DispatchError, Domain};

use crate::proto::{CMD_CREATE_GENERAL_SERVICE, CMD_CREATE_GENERAL_SERVICE_OLD};

/// `CreateGeneralServiceOld` (cmd 4, pre-`[3.0.0]`): no `send_pid`, no payload.
///
/// Returns the raw domain sub-object id assigned by the server. The freshly
/// minted `DomainObject` is kept alive via [`ManuallyDrop`] so the pool can
/// re-open it per request.
pub(crate) fn create_general_service_old(
    creator: &Domain,
) -> Result<u32, CreateGeneralServiceError> {
    let mut result = creator
        .dispatch(CMD_CREATE_GENERAL_SERVICE_OLD)
        .out_objects(1)
        .send()
        .map_err(CreateGeneralServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateGeneralServiceError::MissingObject)?;
    Ok(ManuallyDrop::new(object).object_id().to_raw())
}

/// `CreateGeneralService` (cmd 5, `[3.0.0+]`): `send_pid` + `u64 reserved = 0`.
///
/// Returns the raw domain sub-object id assigned by the server. The freshly
/// minted `DomainObject` is kept alive via [`ManuallyDrop`] so the pool can
/// re-open it per request.
pub(crate) fn create_general_service(creator: &Domain) -> Result<u32, CreateGeneralServiceError> {
    let reserved: u64 = 0;
    // SAFETY: `reserved` is a `Copy` value on the stack, valid until `send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const reserved).cast::<u8>(), size_of::<u64>())
    };
    let mut result = creator
        .dispatch(CMD_CREATE_GENERAL_SERVICE)
        .send_pid()
        .in_raw(in_bytes)
        .out_objects(1)
        .send()
        .map_err(CreateGeneralServiceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateGeneralServiceError::MissingObject)?;
    Ok(ManuallyDrop::new(object).object_id().to_raw())
}

/// Error returned by [`create_general_service`] / [`create_general_service_old`].
#[derive(Debug, thiserror::Error)]
pub enum CreateGeneralServiceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateGeneralService")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("CreateGeneralService response did not include the expected sub-object")]
    MissingObject,
}
