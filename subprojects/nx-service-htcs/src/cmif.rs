//! CMIF protocol operations for the HTCS service.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainRef,
    OutHandleAttr,
    Session,
};

use crate::{
    proto,
    types::{
        AcceptResultsOut,
        ContinueSendOut,
        EndSelectOut,
        FcntlIn,
        HtcsPeerName,
        HtcsSockAddr,
        HtcsTimeVal,
        RecvStartIn,
        SocketResult,
        StartSendOut,
        StartTransferIn,
        TransferResult,
    },
};

/// Sends PID initialization on the manager session (cmd 100).
pub(crate) fn manager_pid_init(domain: DomainRef<'_>) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    // SAFETY: `pid_placeholder` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const pid_placeholder).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    domain
        .dispatch(proto::MANAGER_PID_INIT)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}

/// Sends PID initialization on the monitor session (cmd 101).
pub(crate) fn monitor_pid_init(session: &Session) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    // SAFETY: `pid_placeholder` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const pid_placeholder).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    session
        .dispatch(proto::MONITOR_PID_INIT)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut buf)
        .map(|_| ())
}

/// Gets a peer name (shared implementation for cmds 10 and 11).
pub(crate) fn get_peer_name(
    domain: DomainRef<'_>,
    cmd_id: u32,
) -> Result<HtcsPeerName, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(cmd_id)
        .out_size(size_of::<HtcsPeerName>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<HtcsPeerName>().
    let name = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<HtcsPeerName>()) };
    Ok(name)
}

/// Creates a socket sub-object (cmd 13).
///
/// Returns `(err, socket_object_id)`.
pub(crate) fn create_socket(
    domain: DomainRef<'_>,
    enable_disconnection_emulation: bool,
) -> Result<(i32, u32), CreateSocketError> {
    let input: u8 = if enable_disconnection_emulation { 1 } else { 0 };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<u8>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::CREATE_SOCKET)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateSocketError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<i32>().
    let err = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };

    let object = result
        .take_object(0)
        .ok_or(CreateSocketError::MissingObject)?;
    // The pool re-addresses this id per request via
    // `SessionGuard::open_object_unchecked`, so the close obligation is handed
    // to the `Socket` wrapper rather than discharged here.
    Ok((err, object.into_raw_object_id()))
}

/// Starts a select operation (cmd 130).
///
/// Returns `(task_id, event_handle)`.
pub(crate) fn start_select(
    domain: DomainRef<'_>,
    tv: &HtcsTimeVal,
    read_fds: &[i32],
    write_fds: &[i32],
    except_fds: &[i32],
) -> Result<(u32, u32), StartSelectError> {
    // SAFETY: `tv` is a valid reference; viewing its bytes as a slice is
    // sound, and the slice lives until `.send()` returns.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *tv).cast::<u8>(), size_of::<HtcsTimeVal>())
    };
    // SAFETY: `read_fds`, `write_fds`, `except_fds` are valid `&[i32]`
    // slices; viewing them as byte slices for IN buffers is sound.
    let read_bytes = unsafe {
        core::slice::from_raw_parts(
            read_fds.as_ptr().cast::<u8>(),
            core::mem::size_of_val(read_fds),
        )
    };
    let write_bytes = unsafe {
        core::slice::from_raw_parts(
            write_fds.as_ptr().cast::<u8>(),
            core::mem::size_of_val(write_fds),
        )
    };
    let except_bytes = unsafe {
        core::slice::from_raw_parts(
            except_fds.as_ptr().cast::<u8>(),
            core::mem::size_of_val(except_fds),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::START_SELECT)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .in_buffer(read_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(write_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(except_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(StartSelectError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let task_id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    if result.copy_handles.is_empty() {
        return Err(StartSelectError::MissingEventHandle);
    }

    Ok((task_id, result.copy_handles[0]))
}

/// Ends a select operation (cmd 131).
pub(crate) fn end_select(
    domain: DomainRef<'_>,
    task_id: u32,
    read_fds: &mut [i32],
    write_fds: &mut [i32],
    except_fds: &mut [i32],
) -> Result<EndSelectOut, DispatchError> {
    // SAFETY: `task_id` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const task_id).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `read_fds`, `write_fds`, `except_fds` are valid `&mut [i32]`
    // slices; viewing them as mutable byte slices for OUT buffers is sound.
    let read_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            read_fds.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(read_fds),
        )
    };
    let write_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            write_fds.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(write_fds),
        )
    };
    let except_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            except_fds.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(except_fds),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = domain
        .dispatch(proto::END_SELECT)
        .in_raw(in_bytes)
        .out_size(size_of::<EndSelectOut>())
        .out_buffer(read_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(write_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(except_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<EndSelectOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<EndSelectOut>()) };
    Ok(out)
}

/// Socket close (cmd 0).
pub(crate) fn socket_close(object: DomainObjectRef<'_>) -> Result<SocketResult, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_CLOSE)
        .out_size(size_of::<SocketResult>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<SocketResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SocketResult>()) };
    Ok(out)
}

/// Socket command with HtcsSockAddr input and SocketResult output (cmds 1, 2).
pub(crate) fn socket_cmd_in_address(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    address: &HtcsSockAddr,
) -> Result<SocketResult, DispatchError> {
    // SAFETY: `address` is a valid reference; viewing its bytes as a slice is
    // sound, and the slice lives until `.send()` returns.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *address).cast::<u8>(),
            size_of::<HtcsSockAddr>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<SocketResult>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<SocketResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SocketResult>()) };
    Ok(out)
}

/// Socket command with i32 input and SocketResult output (cmds 3, 7).
pub(crate) fn socket_cmd_in_i32(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    value: i32,
) -> Result<SocketResult, DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<i32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<SocketResult>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<SocketResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SocketResult>()) };
    Ok(out)
}

/// Socket fcntl (cmd 8).
pub(crate) fn socket_fcntl(
    object: DomainObjectRef<'_>,
    command: i32,
    value: i32,
) -> Result<SocketResult, DispatchError> {
    let input = FcntlIn { command, value };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<FcntlIn>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_FCNTL)
        .in_raw(in_bytes)
        .out_size(size_of::<SocketResult>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<SocketResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<SocketResult>()) };
    Ok(out)
}

/// Socket accept start (cmd 9).
///
/// Returns `(task_id, event_handle)`.
pub(crate) fn socket_accept_start(
    object: DomainObjectRef<'_>,
) -> Result<(u32, u32), AcceptStartError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_ACCEPT_START)
        .out_size(size_of::<u32>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(AcceptStartError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let task_id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    if result.copy_handles.is_empty() {
        return Err(AcceptStartError::MissingEventHandle);
    }

    Ok((task_id, result.copy_handles[0]))
}

/// Socket accept results (cmd 10).
///
/// Returns `(AcceptResultsOut, socket_object_id)`.
pub(crate) fn socket_accept_results(
    object: DomainObjectRef<'_>,
    task_id: u32,
) -> Result<(AcceptResultsOut, u32), AcceptResultsError> {
    // SAFETY: `task_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const task_id).cast::<u8>(), size_of::<u32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = object
        .dispatch(proto::SOCKET_ACCEPT_RESULTS)
        .in_raw(in_bytes)
        .out_size(size_of::<AcceptResultsOut>())
        .out_objects(1)
        .send(&mut buf)
        .map_err(AcceptResultsError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<AcceptResultsOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<AcceptResultsOut>()) };

    let accepted = result
        .take_object(0)
        .ok_or(AcceptResultsError::MissingObject)?;
    // Pool re-opens this id per request; keep the server-side object alive
    // beyond this call.
    let raw = accepted.into_raw_object_id();
    Ok((out, raw))
}

/// Socket recv start (cmd 11).
///
/// Returns `(task_id, event_handle)`.
pub(crate) fn socket_recv_start(
    object: DomainObjectRef<'_>,
    mem_size: i32,
    flags: i32,
) -> Result<(u32, u32), RecvStartError> {
    let input = RecvStartIn { mem_size, flags };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<RecvStartIn>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_RECV_START)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(RecvStartError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let task_id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    if result.copy_handles.is_empty() {
        return Err(RecvStartError::MissingEventHandle);
    }

    Ok((task_id, result.copy_handles[0]))
}

/// Socket recv results (cmd 12).
pub(crate) fn socket_recv_results(
    object: DomainObjectRef<'_>,
    task_id: u32,
    buffer: &mut [u8],
) -> Result<TransferResult, DispatchError> {
    // SAFETY: `task_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const task_id).cast::<u8>(), size_of::<u32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_RECV_RESULTS)
        .in_raw(in_bytes)
        .out_size(size_of::<TransferResult>())
        .out_buffer(buffer, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<TransferResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<TransferResult>()) };
    Ok(out)
}

/// Socket command with u32 input and TransferResult output (cmds 16, 19).
pub(crate) fn socket_cmd_in_u32_out_transfer(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    value: u32,
) -> Result<TransferResult, DispatchError> {
    // SAFETY: `value` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const value).cast::<u8>(), size_of::<u32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<TransferResult>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<TransferResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<TransferResult>()) };
    Ok(out)
}

/// Socket start_send (cmd 17).
///
/// Returns `(StartSendOut, event_handle)`.
pub(crate) fn socket_start_send(
    object: DomainObjectRef<'_>,
    size: i64,
    flags: i32,
) -> Result<(StartSendOut, u32), StartSendError> {
    let input = StartTransferIn::new(flags, size);

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StartTransferIn>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_START_SEND)
        .in_raw(in_bytes)
        .out_size(size_of::<StartSendOut>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(StartSendError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<StartSendOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<StartSendOut>()) };

    if result.copy_handles.is_empty() {
        return Err(StartSendError::MissingEventHandle);
    }

    Ok((out, result.copy_handles[0]))
}

/// Socket start_recv (cmd 20).
///
/// Returns `(task_id, event_handle)`.
pub(crate) fn socket_start_recv(
    object: DomainObjectRef<'_>,
    size: i64,
    flags: i32,
) -> Result<(u32, u32), StartRecvError> {
    let input = StartTransferIn::new(flags, size);

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StartTransferIn>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_START_RECV)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(StartRecvError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let task_id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    if result.copy_handles.is_empty() {
        return Err(StartRecvError::MissingEventHandle);
    }

    Ok((task_id, result.copy_handles[0]))
}

/// Socket end_recv (cmd 21).
pub(crate) fn socket_end_recv(
    object: DomainObjectRef<'_>,
    task_id: u32,
    buffer: &mut [u8],
) -> Result<TransferResult, DispatchError> {
    // SAFETY: `task_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const task_id).cast::<u8>(), size_of::<u32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_END_RECV)
        .in_raw(in_bytes)
        .out_size(size_of::<TransferResult>())
        .out_buffer(buffer, BufferAttr::HIPC_AUTO_SELECT)
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<TransferResult>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<TransferResult>()) };
    Ok(out)
}

/// Socket send_start (cmd 22).
///
/// Returns `(task_id, event_handle)`.
pub(crate) fn socket_send_start(
    object: DomainObjectRef<'_>,
    buffer: &[u8],
    flags: i32,
) -> Result<(u32, u32), SendStartError> {
    // SAFETY: `flags` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const flags).cast::<u8>(), size_of::<i32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_SEND_START)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .in_buffer(
            buffer,
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(SendStartError::Dispatch)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let task_id = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    if result.copy_handles.is_empty() {
        return Err(SendStartError::MissingEventHandle);
    }

    Ok((task_id, result.copy_handles[0]))
}

/// Socket continue_send (cmd 23).
pub(crate) fn socket_continue_send(
    object: DomainObjectRef<'_>,
    task_id: u32,
    buffer: &[u8],
) -> Result<ContinueSendOut, DispatchError> {
    // SAFETY: `task_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const task_id).cast::<u8>(), size_of::<u32>()) };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_CONTINUE_SEND)
        .in_raw(in_bytes)
        .out_size(size_of::<ContinueSendOut>())
        .in_buffer(
            buffer,
            BufferAttr::HIPC_AUTO_SELECT.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<ContinueSendOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<ContinueSendOut>()) };
    Ok(out)
}

/// Socket get_primitive (cmd 130).
pub(crate) fn socket_get_primitive(object: DomainObjectRef<'_>) -> Result<i32, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::SOCKET_GET_PRIMITIVE)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<i32>().
    let fd = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };
    Ok(fd)
}

/// Error returned by [`create_socket`].
#[derive(Debug, thiserror::Error)]
pub enum CreateSocketError {
    #[error("failed to dispatch CreateSocket")]
    Dispatch(#[source] DispatchError),
    #[error("CreateSocket response did not include the expected sub-object")]
    MissingObject,
}

/// Error returned by [`start_select`].
#[derive(Debug, thiserror::Error)]
pub enum StartSelectError {
    #[error("failed to dispatch StartSelect")]
    Dispatch(#[source] DispatchError),
    #[error("StartSelect response did not include the expected event handle")]
    MissingEventHandle,
}

/// Error returned by [`socket_accept_start`].
#[derive(Debug, thiserror::Error)]
pub enum AcceptStartError {
    #[error("failed to dispatch AcceptStart")]
    Dispatch(#[source] DispatchError),
    #[error("AcceptStart response did not include the expected event handle")]
    MissingEventHandle,
}

/// Error returned by [`socket_accept_results`].
#[derive(Debug, thiserror::Error)]
pub enum AcceptResultsError {
    #[error("failed to dispatch AcceptResults")]
    Dispatch(#[source] DispatchError),
    #[error("AcceptResults response did not include the expected sub-object")]
    MissingObject,
}

/// Error returned by [`socket_recv_start`].
#[derive(Debug, thiserror::Error)]
pub enum RecvStartError {
    #[error("failed to dispatch RecvStart")]
    Dispatch(#[source] DispatchError),
    #[error("RecvStart response did not include the expected event handle")]
    MissingEventHandle,
}

/// Error returned by [`socket_start_send`].
#[derive(Debug, thiserror::Error)]
pub enum StartSendError {
    #[error("failed to dispatch StartSend")]
    Dispatch(#[source] DispatchError),
    #[error("StartSend response did not include the expected event handle")]
    MissingEventHandle,
}

/// Error returned by [`socket_start_recv`].
#[derive(Debug, thiserror::Error)]
pub enum StartRecvError {
    #[error("failed to dispatch StartRecv")]
    Dispatch(#[source] DispatchError),
    #[error("StartRecv response did not include the expected event handle")]
    MissingEventHandle,
}

/// Error returned by [`socket_send_start`].
#[derive(Debug, thiserror::Error)]
pub enum SendStartError {
    #[error("failed to dispatch SendStart")]
    Dispatch(#[source] DispatchError),
    #[error("SendStart response did not include the expected event handle")]
    MissingEventHandle,
}
