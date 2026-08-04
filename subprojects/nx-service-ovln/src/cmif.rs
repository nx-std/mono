//! CMIF protocol operations for the overlay notification service.

use nx_sf::{
    cmif,
    ipc::Handle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
        Session,
    },
};

use crate::{
    proto,
    types::{
        OvlnQueueAttribute,
        OvlnRawMessage,
        OvlnSendOption,
        OvlnSourceName,
        ReceiveWithTickOut,
    },
};

fn dispatch_in<T>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: &T,
) -> Result<(), DispatchInError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchInError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out<T>(session: BorrowedSessionHandle<'_>, cmd_id: u32) -> Result<T, DispatchOutError>
where
    T: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutError::SendRequest)?;

    let resp = cmif::parse_response::<&T>(&buf).map_err(DispatchOutError::ParseResponse)?;

    Ok(*resp.payload)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Opens a receiver sub-object.
pub fn rcv_open_receiver(session: BorrowedSessionHandle<'_>) -> Result<Session, OpenReceiverError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::RCV_OPEN_RECEIVER).build();
    req.send(&mut buf, session)
        .map_err(OpenReceiverError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenReceiverError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenReceiverError::MissingHandle);
    };

    // SAFETY: the handle comes from a successful `OpenReceiver` response;
    // ownership transfers to the new `Session`.
    let handle = Handle::from_raw_unchecked(raw_handle);

    Ok(Session::new(
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // Session below becomes its sole owner.
        OwnedSessionHandle::from_handle_unchecked(handle),
        0,
    ))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenReceiverError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected handle.
    #[error("missing receiver handle in response")]
    MissingHandle,
}

/// Adds a source to the receiver.
pub fn receiver_add_source(
    session: BorrowedSessionHandle<'_>,
    name: &OvlnSourceName,
) -> Result<(), DispatchInError> {
    dispatch_in(session, proto::RECEIVER_ADD_SOURCE, name)
}

/// Removes a source from the receiver.
pub fn receiver_remove_source(
    session: BorrowedSessionHandle<'_>,
    name: &OvlnSourceName,
) -> Result<(), DispatchInError> {
    dispatch_in(session, proto::RECEIVER_REMOVE_SOURCE, name)
}

/// Gets the receive event handle (copy handle).
pub fn receiver_get_receive_event_handle(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, GetReceiveEventHandleError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::RECEIVER_GET_RECEIVE_EVENT_HANDLE).build();
    req.send(&mut buf, session)
        .map_err(GetReceiveEventHandleError::SendRequest)?;

    let resp =
        cmif::parse_response::<()>(&buf).map_err(GetReceiveEventHandleError::ParseResponse)?;

    let Some(&raw_handle) = resp.copy_handles.first() else {
        return Err(GetReceiveEventHandleError::MissingHandle);
    };

    Ok(raw_handle)
}

#[derive(Debug, thiserror::Error)]
pub enum GetReceiveEventHandleError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected handle.
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Receives a message.
pub fn receiver_receive(
    session: BorrowedSessionHandle<'_>,
) -> Result<OvlnRawMessage, DispatchOutError> {
    dispatch_out::<OvlnRawMessage>(session, proto::RECEIVER_RECEIVE)
}

/// Receives a message with a system tick.
pub fn receiver_receive_with_tick(
    session: BorrowedSessionHandle<'_>,
) -> Result<ReceiveWithTickOut, DispatchOutError> {
    dispatch_out::<ReceiveWithTickOut>(session, proto::RECEIVER_RECEIVE_WITH_TICK)
}

/// Opens a sender sub-object.
pub fn snd_open_sender(
    session: BorrowedSessionHandle<'_>,
    name: &OvlnSourceName,
    attribute: &OvlnQueueAttribute,
) -> Result<Session, OpenSenderError> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    struct OpenSenderIn {
        name: OvlnSourceName,
        attribute: OvlnQueueAttribute,
    }

    let input = OpenSenderIn {
        name: *name,
        attribute: *attribute,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SND_OPEN_SENDER)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenSenderError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSenderError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenSenderError::MissingHandle);
    };

    // SAFETY: the handle comes from a successful `OpenSender` response;
    // ownership transfers to the new `Session`.
    let handle = Handle::from_raw_unchecked(raw_handle);

    Ok(Session::new(
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // Session below becomes its sole owner.
        OwnedSessionHandle::from_handle_unchecked(handle),
        0,
    ))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenSenderError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected handle.
    #[error("missing sender handle in response")]
    MissingHandle,
}

/// Sends a message.
pub fn sender_send(
    session: BorrowedSessionHandle<'_>,
    option: &OvlnSendOption,
    message: &OvlnRawMessage,
) -> Result<(), DispatchInError> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
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
pub fn sender_get_unreceived_message_count(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, DispatchOutError> {
    dispatch_out::<u32>(session, proto::SENDER_GET_UNRECEIVED_MESSAGE_COUNT)
}
