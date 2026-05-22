//! CMIF protocol operations for the clkrst service.

use core::{
    mem::{size_of, size_of_val},
    ptr,
};

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    proto,
    types::{ClockRatesListType, PcvModuleId},
};

/// Opens a [`ClkrstSession`](crate::ClkrstSession) for the given module.
pub fn open_session(
    session: SessionHandle,
    module_id: PcvModuleId,
    unk: u32,
) -> Result<SessionHandle, OpenSessionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
        .data_size(size_of::<[u32; 2]>())
        .build();
    req.write_to(&mut buf)
        .map_err(OpenSessionError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<[u32; 2]>()` bytes.
    unsafe {
        let data_ptr = buf.as_array_mut().as_mut_ptr().cast::<u32>();
        ptr::write_unaligned(data_ptr, module_id.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
    }

    ipc::send_sync_request(&mut buf, session).map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSessionError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Sets the clock rate in Hz.
pub fn set_clock_rate(session: SessionHandle, hz: u32) -> Result<(), SetClockRateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SET_CLOCK_RATE)
        .data_size(size_of::<u32>())
        .build();
    req.write_to(&mut buf)
        .map_err(SetClockRateError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u32>(), hz) };

    ipc::send_sync_request(&mut buf, session).map_err(SetClockRateError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetClockRateError::ParseResponse)?;

    Ok(())
}

/// Gets the current clock rate in Hz.
pub fn get_clock_rate(session: SessionHandle) -> Result<u32, GetClockRateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_CLOCK_RATE).build();
    req.write_to(&mut buf)
        .map_err(GetClockRateError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetClockRateError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(GetClockRateError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<u32>()` bytes.
    let hz = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(hz)
}

/// Result of [`get_possible_clock_rates`].
pub struct PossibleClockRates {
    /// The type of the clock rates list.
    pub list_type: ClockRatesListType,
    /// The number of valid entries written to the output buffer.
    pub count: i32,
}

/// Gets the list of possible clock rates for this session.
///
/// Fills `rates` with up to `rates.len()` entries and returns the list
/// type and actual count.
pub fn get_possible_clock_rates(
    session: SessionHandle,
    rates: &mut [u32],
) -> Result<PossibleClockRates, GetPossibleClockRatesError> {
    let max_count: i32 = rates.len() as i32;

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_POSSIBLE_CLOCK_RATES)
        .data_size(size_of::<i32>())
        .add_out_auto_buffer(
            rates.as_mut_ptr().cast::<u8>(),
            size_of_val(rates),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetPossibleClockRatesError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<i32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<i32>(), max_count) };

    ipc::send_sync_request(&mut buf, session).map_err(GetPossibleClockRatesError::SendRequest)?;

    // Response inline data: { i32 type, i32 count }.
    let resp = cmif::parse_response_bytes(&buf, size_of::<[i32; 2]>())
        .map_err(GetPossibleClockRatesError::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<[i32; 2]>()` bytes.
    let raw_type = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>().add(1)) };

    let list_type = ClockRatesListType::from_raw(raw_type)
        .ok_or(GetPossibleClockRatesError::UnknownListType(raw_type))?;

    Ok(PossibleClockRates { list_type, count })
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
    /// Response did not include the expected session handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`set_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum SetClockRateError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`get_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum GetClockRateError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`get_possible_clock_rates`].
#[derive(Debug, thiserror::Error)]
pub enum GetPossibleClockRatesError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Unknown clock rates list type.
    #[error("unknown clock rates list type: {0}")]
    UnknownListType(i32),
}
