//! CMIF protocol operations for the clkrst service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION)
        .data_size(size_of::<[u32; 2]>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for two u32.
    unsafe {
        let data_ptr = req.data.as_ptr().cast::<u32>().cast_mut();
        ptr::write_unaligned(data_ptr, module_id.as_raw());
        ptr::write_unaligned(data_ptr.add(1), unk);
    }

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Sets the clock rate in Hz.
pub fn set_clock_rate(session: SessionHandle, hz: u32) -> Result<(), SetClockRateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_CLOCK_RATE)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), hz);
    }

    ipc::send_sync_request(session).map_err(SetClockRateError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetClockRateError::ParseResponse)?;

    Ok(())
}

/// Gets the current clock rate in Hz.
pub fn get_clock_rate(session: SessionHandle) -> Result<u32, GetClockRateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_CLOCK_RATE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetClockRateError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(GetClockRateError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let max_count: i32 = rates.len() as i32;

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_POSSIBLE_CLOCK_RATES)
        .data_size(size_of::<i32>())
        .out_auto_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for i32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<i32>().cast_mut(), max_count);
    }

    req.add_out_auto_buffer(
        rates.as_mut_ptr().cast::<u8>(),
        size_of_val(rates),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetPossibleClockRatesError::SendRequest)?;

    // Response inline data: { i32 type, i32 count }.
    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<[i32; 2]>()) }
        .map_err(GetPossibleClockRatesError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for [i32; 2].
    let raw_type = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };
    let count = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>().add(1)) };

    let list_type = ClockRatesListType::from_raw(raw_type)
        .ok_or(GetPossibleClockRatesError::UnknownListType(raw_type))?;

    Ok(PossibleClockRates { list_type, count })
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`set_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum SetClockRateError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum GetClockRateError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_possible_clock_rates`].
#[derive(Debug, thiserror::Error)]
pub enum GetPossibleClockRatesError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("unknown clock rates list type: {0}")]
    UnknownListType(i32),
}
