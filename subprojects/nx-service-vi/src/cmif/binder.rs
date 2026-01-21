//! CMIF operations for IHOSBinderDriverRelay.
//!
//! Used for Binder transactions with IGraphicBufferProducer.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::{
    ipc::{self, Handle as SessionHandle},
    raw::Handle as RawHandle,
};

use crate::{proto::binder_cmds, types::BinderObjectId};

/// Performs a parcel transaction.
///
/// Uses TransactParcelAuto (cmd 3) on 3.0.0+, TransactParcel (cmd 0) otherwise.
pub fn transact_parcel(
    session: SessionHandle,
    binder_id: BinderObjectId,
    code: u32,
    in_data: &[u8],
    out_data: &mut [u8],
    flags: u32,
) -> Result<(), TransactParcelError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // Always use auto mode (3.0.0+) for simplicity
    // If supporting older firmware is needed, add version check
    let cmd_id = binder_cmds::TRANSACT_PARCEL_AUTO;

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(12) // session_id(4) + code(4) + flags(4)
        .in_auto_buffers(1)
        .out_auto_buffers(1)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        session_id: i32,
        code: u32,
        flags: u32,
    }

    let input = Input {
        session_id: binder_id.to_raw(),
        code,
        flags,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add auto-select buffers (Normal mode)
    req.add_in_auto_buffer(in_data.as_ptr(), in_data.len(), BufferMode::Normal);
    req.add_out_auto_buffer(out_data.as_mut_ptr(), out_data.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(TransactParcelError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(TransactParcelError::ParseResponse)?;

    Ok(())
}

/// Adjusts the reference count on a binder object.
///
/// # Arguments
/// * `addval` - Amount to add (+1 to increase, -1 to decrease)
/// * `type_` - Reference type (0 for weak, 1 for strong)
pub fn adjust_refcount(
    session: SessionHandle,
    binder_id: BinderObjectId,
    addval: i32,
    type_: i32,
) -> Result<(), AdjustRefcountError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(binder_cmds::ADJUST_REFCOUNT)
        .data_size(12) // session_id(4) + addval(4) + type(4)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        session_id: i32,
        addval: i32,
        type_: i32,
    }

    let input = Input {
        session_id: binder_id.to_raw(),
        addval,
        type_,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(AdjustRefcountError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(AdjustRefcountError::ParseResponse)?;

    Ok(())
}

/// Gets a native handle from the binder.
pub fn get_native_handle(
    session: SessionHandle,
    binder_id: BinderObjectId,
    inval: u32,
) -> Result<RawHandle, GetNativeHandleError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(binder_cmds::GET_NATIVE_HANDLE)
        .data_size(8) // session_id(4) + inval(4)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        session_id: i32,
        inval: u32,
    }

    let input = Input {
        session_id: binder_id.to_raw(),
        inval,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(GetNativeHandleError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetNativeHandleError::ParseResponse)?;

    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetNativeHandleError::MissingHandle)?;

    Ok(handle)
}

/// Error from [`transact_parcel`].
#[derive(Debug, thiserror::Error)]
pub enum TransactParcelError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`adjust_refcount`].
#[derive(Debug, thiserror::Error)]
pub enum AdjustRefcountError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`get_native_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetNativeHandleError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing handle in response.
    #[error("missing handle in response")]
    MissingHandle,
}
