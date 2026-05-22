//! CMIF protocol operations for the PL (shared font) service.

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::GetSharedFontOut};

/// Requests loading of a shared font into shared memory.
pub fn request_load(session: SessionHandle, font_type: u32) -> Result<(), RequestLoadError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::REQUEST_LOAD)
        .with_data_value(&font_type)
        .build();
    req.write_to(&mut buf)
        .map_err(RequestLoadError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RequestLoadError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RequestLoadError::ParseResponse)?;

    Ok(())
}

/// Gets the load state of a shared font.
///
/// Returns the load state: 0 = not loaded, 1 = loaded.
pub fn get_load_state(session: SessionHandle, font_type: u32) -> Result<u32, GetLoadStateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_LOAD_STATE)
        .with_data_value(&font_type)
        .build();
    req.write_to(&mut buf)
        .map_err(GetLoadStateError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetLoadStateError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(GetLoadStateError::ParseResponse)?;

    let state = *resp.payload;

    Ok(state)
}

/// Gets the size of a shared font in bytes.
pub fn get_size(session: SessionHandle, font_type: u32) -> Result<u32, GetSizeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_SIZE)
        .with_data_value(&font_type)
        .build();
    req.write_to(&mut buf).map_err(GetSizeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetSizeError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(GetSizeError::ParseResponse)?;

    let size = *resp.payload;

    Ok(size)
}

/// Gets the byte offset of a shared font within shared memory.
pub fn get_shared_memory_address_offset(
    session: SessionHandle,
    font_type: u32,
) -> Result<u32, GetSharedMemoryAddressOffsetError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_SHARED_MEMORY_ADDRESS_OFFSET)
        .with_data_value(&font_type)
        .build();
    req.write_to(&mut buf)
        .map_err(GetSharedMemoryAddressOffsetError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetSharedMemoryAddressOffsetError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf)
        .map_err(GetSharedMemoryAddressOffsetError::ParseResponse)?;

    let offset = *resp.payload;

    Ok(offset)
}

/// Gets the shared memory native handle (copy handle).
pub fn get_shared_memory_native_handle(
    session: SessionHandle,
) -> Result<u32, GetSharedMemoryNativeHandleError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_SHARED_MEMORY_NATIVE_HANDLE).build();
    req.write_to(&mut buf)
        .map_err(GetSharedMemoryNativeHandleError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetSharedMemoryNativeHandleError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf)
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_SHARED_FONT)
        .with_data_value(&language_code)
        .add_output_buffer_raw(
            types.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(types),
            BufferMode::Normal,
        )
        .add_output_buffer_raw(
            offsets.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(offsets),
            BufferMode::Normal,
        )
        .add_output_buffer_raw(
            sizes.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(sizes),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetSharedFontError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetSharedFontError::SendRequest)?;

    let resp = cmif::parse_response::<&GetSharedFontOut>(&buf)
        .map_err(GetSharedFontError::ParseResponse)?;

    let out = *resp.payload;

    Ok(out)
}

/// Error returned by [`request_load`].
#[derive(Debug, thiserror::Error)]
pub enum RequestLoadError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_load_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetLoadStateError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_size`].
#[derive(Debug, thiserror::Error)]
pub enum GetSizeError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_shared_memory_address_offset`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryAddressOffsetError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_shared_memory_native_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryNativeHandleError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected copy handle.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// Error returned by [`get_shared_font`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedFontError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
