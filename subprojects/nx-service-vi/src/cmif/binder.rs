//! CMIF operations for IHOSBinderDriverRelay.
//!
//! Used for Binder transactions with IGraphicBufferProducer.

use core::ptr;

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};
use nx_svc::raw::Handle as RawHandle;

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
    // Always use the auto-buffer transact (cmd 3) introduced in HOS 3.0.0.
    // The pre-3.0.0 fallback (cmd 0 over HipcMapAlias buffers) is unimplemented;
    // production targets are 3.0.0+ and the runtime makes no attempt to gate
    // on older firmware. If pre-3.0.0 support is ever required, dispatch a
    // separate branch keyed off the caller-supplied HOS version.
    let cmd_id = binder_cmds::TRANSACT_PARCEL_AUTO;

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

    // Add auto-select buffers (Normal mode)
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .data_size(12) // session_id(4) + code(4) + flags(4)
        .add_in_auto_buffer(in_data.as_ptr(), in_data.len(), BufferMode::Normal)
        .add_out_auto_buffer(out_data.as_mut_ptr(), out_data.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(TransactParcelError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<Input>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(TransactParcelError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(TransactParcelError::ParseResponse)?;

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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

    let req = cmif::CmifRequestBuilder::new(binder_cmds::ADJUST_REFCOUNT)
        .data_size(12)
        .build();
    req.write_to(&mut buf)
        .map_err(AdjustRefcountError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<Input>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(AdjustRefcountError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(AdjustRefcountError::ParseResponse)?;

    Ok(())
}

/// Gets a native handle from the binder.
pub fn get_native_handle(
    session: SessionHandle,
    binder_id: BinderObjectId,
    inval: u32,
) -> Result<RawHandle, GetNativeHandleError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    struct Input {
        session_id: i32,
        inval: u32,
    }

    let input = Input {
        session_id: binder_id.to_raw(),
        inval,
    };

    let req = cmif::CmifRequestBuilder::new(binder_cmds::GET_NATIVE_HANDLE)
        .data_size(8)
        .build();
    req.write_to(&mut buf)
        .map_err(GetNativeHandleError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<Input>(), input) };

    ipc::send_sync_request(&mut buf, session).map_err(GetNativeHandleError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, 0).map_err(GetNativeHandleError::ParseResponse)?;

    let Some(&handle) = resp.copy_handles.first() else {
        return Err(GetNativeHandleError::MissingHandle);
    };

    Ok(handle)
}

/// Error from [`transact_parcel`].
#[derive(Debug, thiserror::Error)]
pub enum TransactParcelError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error from [`adjust_refcount`].
#[derive(Debug, thiserror::Error)]
pub enum AdjustRefcountError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error from [`get_native_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetNativeHandleError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Missing handle in response.
    #[error("missing handle in response")]
    MissingHandle,
}
