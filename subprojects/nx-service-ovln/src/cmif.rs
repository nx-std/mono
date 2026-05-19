//! CMIF protocol operations for the overlay notification service.

use core::{mem::size_of, ptr};

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

fn dispatch_in<T: Copy>(session: Handle, cmd_id: u32, value: &T) -> Result<(), DispatchInError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, cmd_id)
            .data_size(size_of::<T>())
            .send()
            .map_err(DispatchInError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<T>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<T>(), *value);
        }
    }

    ipc::send_sync_request(session).map_err(DispatchInError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(DispatchInError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInError {
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

fn dispatch_out<T: Copy>(session: Handle, cmd_id: u32) -> Result<T, DispatchOutError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, cmd_id)
            .send()
            .map_err(DispatchOutError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchOutError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<T>())
        .map_err(DispatchOutError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<T>()` bytes.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<T>()) };

    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutError {
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

// ---------------------------------------------------------------------------
// Receiver manager commands
// ---------------------------------------------------------------------------

/// Opens a receiver sub-object.
pub fn rcv_open_receiver(session: Handle) -> Result<Session, OpenReceiverError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::RCV_OPEN_RECEIVER)
            .send()
            .map_err(OpenReceiverError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(OpenReceiverError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenReceiverError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenReceiverError::MissingHandle);
    };

    // SAFETY: the handle comes from a successful `OpenReceiver` response;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenReceiverError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected handle.
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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::RECEIVER_GET_RECEIVE_EVENT_HANDLE)
            .send()
            .map_err(GetReceiveEventHandleError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetReceiveEventHandleError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, 0).map_err(GetReceiveEventHandleError::ParseResponse)?;

    let Some(&raw_handle) = resp.copy_handles.first() else {
        return Err(GetReceiveEventHandleError::MissingHandle);
    };

    Ok(raw_handle)
}

#[derive(Debug, thiserror::Error)]
pub enum GetReceiveEventHandleError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected handle.
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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SND_OPEN_SENDER)
            .data_size(size_of::<OpenSenderIn>())
            .send()
            .map_err(OpenSenderError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<OpenSenderIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<OpenSenderIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(OpenSenderError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenSenderError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenSenderError::MissingHandle);
    };

    // SAFETY: the handle comes from a successful `OpenSender` response;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum OpenSenderError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected handle.
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
