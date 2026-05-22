//! IECommerceInterface CMIF commands.

use core::mem::size_of;

use nx_sf::service::{OutHandleAttr, Session};

use super::app_manager::{AsyncCommandError, AsyncOut};
use crate::{proto, types::AccountUid};

/// RequestLinkDevice (cmd 0).
pub(crate) fn request_link_device(
    service: &Session,
    uid: AccountUid,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ECOMMERCE_REQUEST_LINK_DEVICE)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}

/// RequestSyncRights (cmd 3).
pub(crate) fn request_sync_rights(service: &Session) -> Result<AsyncOut, AsyncCommandError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ECOMMERCE_REQUEST_SYNC_RIGHTS)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}

/// RequestUnlinkDevice (cmd 4).
pub(crate) fn request_unlink_device(
    service: &Session,
    uid: AccountUid,
) -> Result<AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const uid).cast::<u8>(), size_of::<AccountUid>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::ECOMMERCE_REQUEST_UNLINK_DEVICE)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}
