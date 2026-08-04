//! CMIF protocol operations for the clkrst service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        OutputBuffer,
    },
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
};
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    types::{
        ClockRatesListType,
        PcvModuleId,
    },
};

#[repr(C, packed)]
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
struct OpenSessionIn {
    module_id: u32,
    unk: u32,
}

/// Inline response payload for
/// [`GetPossibleClockRates`](crate::proto::GET_POSSIBLE_CLOCK_RATES).
#[repr(C)]
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
struct GetPossibleClockRatesOut {
    list_type: i32,
    count: i32,
}

/// Opens a [`ClkrstSession`](crate::ClkrstSession) for the given module.
pub fn open_session(
    session: BorrowedSessionHandle<'_>,
    module_id: PcvModuleId,
    unk: u32,
) -> Result<OwnedSessionHandle, OpenSessionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let input = OpenSessionIn {
        module_id: module_id.as_raw(),
        unk,
    };
    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSessionError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Sets the clock rate in Hz.
pub fn set_clock_rate(
    session: BorrowedSessionHandle<'_>,
    hz: u32,
) -> Result<(), SetClockRateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SET_CLOCK_RATE)
        .with_data_value(&hz)
        .build();
    req.send(&mut buf, session)
        .map_err(SetClockRateError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetClockRateError::ParseResponse)?;

    Ok(())
}

/// Gets the current clock rate in Hz.
pub fn get_clock_rate(session: BorrowedSessionHandle<'_>) -> Result<u32, GetClockRateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_CLOCK_RATE).build();
    req.send(&mut buf, session)
        .map_err(GetClockRateError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(GetClockRateError::ParseResponse)?;

    Ok(*resp.payload)
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
    session: BorrowedSessionHandle<'_>,
    rates: &mut [u32],
) -> Result<PossibleClockRates, GetPossibleClockRatesError> {
    let max_count: i32 = rates.len() as i32;

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_POSSIBLE_CLOCK_RATES)
        .with_data_value(&max_count)
        .add_out_auto_buffer(OutputBuffer::new(rates.as_mut_bytes(), BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetPossibleClockRatesError::SendRequest)?;

    // Response inline data: { i32 type, i32 count }.
    let resp = cmif::parse_response::<&GetPossibleClockRatesOut>(&buf)
        .map_err(GetPossibleClockRatesError::ParseResponse)?;

    let raw_type = resp.payload.list_type;
    let count = resp.payload.count;

    let list_type = ClockRatesListType::from_raw(raw_type)
        .ok_or(GetPossibleClockRatesError::UnknownListType(raw_type))?;

    Ok(PossibleClockRates { list_type, count })
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not include the expected session handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`set_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum SetClockRateError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_clock_rate`].
#[derive(Debug, thiserror::Error)]
pub enum GetClockRateError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_possible_clock_rates`].
#[derive(Debug, thiserror::Error)]
pub enum GetPossibleClockRatesError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Unknown clock rates list type.
    #[error("unknown clock rates list type: {0}")]
    UnknownListType(i32),
}
