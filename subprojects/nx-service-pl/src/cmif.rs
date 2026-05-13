//! CMIF protocol operations for the PL (shared font) service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::GetSharedFontOut};

/// Requests loading of a shared font into shared memory.
pub fn request_load(session: SessionHandle, font_type: u32) -> Result<(), RequestLoadError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::REQUEST_LOAD)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), font_type);
    }

    ipc::send_sync_request(session).map_err(RequestLoadError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RequestLoadError::ParseResponse)?;

    Ok(())
}

/// Gets the load state of a shared font.
///
/// Returns the load state: 0 = not loaded, 1 = loaded.
pub fn get_load_state(session: SessionHandle, font_type: u32) -> Result<u32, GetLoadStateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_LOAD_STATE)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), font_type);
    }

    ipc::send_sync_request(session).map_err(GetLoadStateError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(GetLoadStateError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let state = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(state)
}

/// Gets the size of a shared font in bytes.
pub fn get_size(session: SessionHandle, font_type: u32) -> Result<u32, GetSizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SIZE)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), font_type);
    }

    ipc::send_sync_request(session).map_err(GetSizeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(GetSizeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(size)
}

/// Gets the byte offset of a shared font within shared memory.
pub fn get_shared_memory_address_offset(
    session: SessionHandle,
    font_type: u32,
) -> Result<u32, GetSharedMemoryAddressOffsetError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SHARED_MEMORY_ADDRESS_OFFSET)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), font_type);
    }

    ipc::send_sync_request(session).map_err(GetSharedMemoryAddressOffsetError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(GetSharedMemoryAddressOffsetError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let offset = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(offset)
}

/// Gets the shared memory native handle (copy handle).
pub fn get_shared_memory_native_handle(
    session: SessionHandle,
) -> Result<u32, GetSharedMemoryNativeHandleError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SHARED_MEMORY_NATIVE_HANDLE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSharedMemoryNativeHandleError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSharedMemoryNativeHandleError::ParseResponse)?;

    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryNativeHandleError::MissingHandle)?;

    Ok(handle)
}

/// Gets shared fonts for a language code.
///
/// Writes font type IDs into `types`, byte offsets into `offsets`, and byte
/// sizes into `sizes`. All three buffers must have the same length (typically
/// [`SharedFontType::TOTAL`](crate::SharedFontType::TOTAL) elements).
///
/// Returns [`GetSharedFontOut`] indicating whether fonts are loaded and how
/// many entries were written.
pub fn get_shared_font(
    session: SessionHandle,
    language_code: u64,
    types: &mut [u32],
    offsets: &mut [u32],
    sizes: &mut [u32],
) -> Result<GetSharedFontOut, GetSharedFontError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SHARED_FONT)
        .data_size(size_of::<u64>())
        .out_buffers(3)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), language_code);
    }

    req.add_out_buffer(
        types.as_mut_ptr().cast::<u8>(),
        size_of_val(types),
        BufferMode::Normal,
    );
    req.add_out_buffer(
        offsets.as_mut_ptr().cast::<u8>(),
        size_of_val(offsets),
        BufferMode::Normal,
    );
    req.add_out_buffer(
        sizes.as_mut_ptr().cast::<u8>(),
        size_of_val(sizes),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetSharedFontError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<GetSharedFontOut>()) }
        .map_err(GetSharedFontError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for GetSharedFontOut.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<GetSharedFontOut>()) };

    Ok(out)
}

/// Error returned by [`request_load`].
#[derive(Debug, thiserror::Error)]
pub enum RequestLoadError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_load_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetLoadStateError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_size`].
#[derive(Debug, thiserror::Error)]
pub enum GetSizeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_shared_memory_address_offset`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryAddressOffsetError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_shared_memory_native_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryNativeHandleError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected copy handle.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// Error returned by [`get_shared_font`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedFontError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
