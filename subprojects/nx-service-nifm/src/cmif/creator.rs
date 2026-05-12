//! `nifm:u` / `nifm:s` / `nifm:a` static-service commands — used to spawn the
//! `IGeneralService` sub-object on the converted-to-domain creator session.

use core::mem::size_of;

use nx_sf::service::{DispatchError, Domain};

use crate::proto::{CMD_CREATE_GENERAL_SERVICE, CMD_CREATE_GENERAL_SERVICE_OLD};

/// `CreateGeneralServiceOld` (cmd 4, pre-`[3.0.0]`): no `send_pid`, no payload.
///
/// Returns the raw domain sub-object id assigned by the server.
pub(crate) fn create_general_service_old(
    creator: &Domain,
) -> Result<u32, CreateGeneralServiceError> {
    let result = creator
        .dispatch(CMD_CREATE_GENERAL_SERVICE_OLD)
        .out_objects(1)
        .send()
        .map_err(CreateGeneralServiceError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(CreateGeneralServiceError::MissingObject);
    }
    Ok(result.objects[0])
}

/// `CreateGeneralService` (cmd 5, `[3.0.0+]`): `send_pid` + `u64 reserved = 0`.
///
/// Returns the raw domain sub-object id assigned by the server.
pub(crate) fn create_general_service(creator: &Domain) -> Result<u32, CreateGeneralServiceError> {
    let reserved: u64 = 0;
    // SAFETY: `reserved` lives on the stack until `send()` returns.
    let result = unsafe {
        creator
            .dispatch(CMD_CREATE_GENERAL_SERVICE)
            .send_pid()
            .in_raw((&raw const reserved).cast::<u8>(), size_of::<u64>())
            .out_objects(1)
            .send()
            .map_err(CreateGeneralServiceError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(CreateGeneralServiceError::MissingObject);
    }
    Ok(result.objects[0])
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
