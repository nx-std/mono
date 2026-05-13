//! CMIF protocol operations for the overlay notification service.

use core::ptr;

use nx_sf::{cmif, service::Session};
use nx_svc::ipc::{self, Handle};

use crate::{
    proto,
    types::{
        OvlnQueueAttribute, OvlnRawMessage, OvlnSendOption, OvlnSourceName, ReceiveWithTickOut,
    },
};

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_in<T>(session: Handle, cmd_id: u32, value: &T) -> Result<(), DispatchInError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<T>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for T.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<T>().cast_mut(), ptr::read(value));
    }

    ipc::send_sync_request(session).map_err(DispatchInError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DispatchInError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out<T: Copy>(session: Handle, cmd_id: u32) -> Result<T, DispatchOutError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<T>()) }
        .map_err(DispatchOutError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for T.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<T>()) };

    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// Receiver manager commands
// ---------------------------------------------------------------------------

/// Opens a receiver sub-object.
pub fn rcv_open_receiver(session: Handle) -> Result<Session, OpenReceiverError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::RCV_OPEN_RECEIVER).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(OpenReceiverError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenReceiverError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenReceiverError::MissingHandle)?;

    // SAFETY: the handle comes from a successful `OpenReceiver` response;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenReceiverError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing receiver handle in response")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// Receiver sub-object commands
// ---------------------------------------------------------------------------

/// Adds a source to the receiver.
pub fn receiver_add_source(session: Handle, name: &OvlnSourceName) -> Result<(), DispatchInError> {
    dispatch_in(session, proto::RECEIVER_ADD_SOURCE, name)
}

/// Removes a source from the receiver.
pub fn receiver_remove_source(
    session: Handle,
    name: &OvlnSourceName,
) -> Result<(), DispatchInError> {
    dispatch_in(session, proto::RECEIVER_REMOVE_SOURCE, name)
}

/// Gets the receive event handle (copy handle).
pub fn receiver_get_receive_event_handle(
    session: Handle,
) -> Result<u32, GetReceiveEventHandleError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::RECEIVER_GET_RECEIVE_EVENT_HANDLE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetReceiveEventHandleError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetReceiveEventHandleError::ParseResponse)?;

    let raw_handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetReceiveEventHandleError::MissingHandle)?;

    Ok(raw_handle)
}

#[derive(Debug, thiserror::Error)]
pub enum GetReceiveEventHandleError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Receives a message.
pub fn receiver_receive(session: Handle) -> Result<OvlnRawMessage, DispatchOutError> {
    dispatch_out::<OvlnRawMessage>(session, proto::RECEIVER_RECEIVE)
}

/// Receives a message with a system tick.
pub fn receiver_receive_with_tick(session: Handle) -> Result<ReceiveWithTickOut, DispatchOutError> {
    dispatch_out::<ReceiveWithTickOut>(session, proto::RECEIVER_RECEIVE_WITH_TICK)
}

// ---------------------------------------------------------------------------
// Sender manager commands
// ---------------------------------------------------------------------------

/// Opens a sender sub-object.
pub fn snd_open_sender(
    session: Handle,
    name: &OvlnSourceName,
    attribute: &OvlnQueueAttribute,
) -> Result<Session, OpenSenderError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct OpenSenderIn {
        name: OvlnSourceName,
        attribute: OvlnQueueAttribute,
    }

    let input = OpenSenderIn {
        name: *name,
        attribute: *attribute,
    };

    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SND_OPEN_SENDER)
        .data_size(size_of::<OpenSenderIn>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for OpenSenderIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<OpenSenderIn>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(OpenSenderError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSenderError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSenderError::MissingHandle)?;

    // SAFETY: the handle comes from a successful `OpenSender` response;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenSenderError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing sender handle in response")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// Sender sub-object commands
// ---------------------------------------------------------------------------

/// Sends a message.
pub fn sender_send(
    session: Handle,
    option: &OvlnSendOption,
    message: &OvlnRawMessage,
) -> Result<(), DispatchInError> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SendIn {
        option: OvlnSendOption,
        message: OvlnRawMessage,
    }

    let input = SendIn {
        option: *option,
        message: *message,
    };

    dispatch_in(session, proto::SENDER_SEND, &input)
}

/// Gets the count of unreceived messages.
pub fn sender_get_unreceived_message_count(session: Handle) -> Result<u32, DispatchOutError> {
    dispatch_out::<u32>(session, proto::SENDER_GET_UNRECEIVED_MESSAGE_COUNT)
}
