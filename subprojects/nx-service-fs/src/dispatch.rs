use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use nx_sys_thread_tls::IpcBuffer;
use zerocopy::IntoBytes as _;

/// Runs `f` against this thread's IPC buffer.
///
/// Borrowing the buffer twice on one thread would put two `&mut` on the same
/// TLS bytes, so every command must know that nobody else holds it. Acquiring
/// it here and nowhere else turns that from a claim each command has to make
/// into one a reader can check: reaching the buffer means calling this
/// function, and no command body calls it again while `f` is running.
///
/// The borrow ends when `f` returns, so whatever `f` needs from the response
/// must be copied out rather than returned by reference.
pub(crate) fn with_ipc_buffer<R>(f: impl FnOnce(&mut IpcBuffer) -> R) -> R {
    // SAFETY: no other `IpcBuffer` is live on this thread. The token is created
    // here and dropped before returning, and `f` cannot make a second one
    // without calling back into this function, which no command body does.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    f(&mut buf)
}

pub(crate) fn dispatch_no_io(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<(), DispatchError> {
    with_ipc_buffer(|ipc_buf| {
        object
            .dispatch(cmd_id)
            .context(ctx)
            .send(ipc_buf)
            .map(|_| ())
    })
}

pub(crate) fn dispatch_in<I>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
    input: I,
) -> Result<(), DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
{
    with_ipc_buffer(|ipc_buf| {
        object
            .dispatch(cmd_id)
            .context(ctx)
            .in_raw(input.as_bytes())
            .send(ipc_buf)
            .map(|_| ())
    })
}

pub(crate) fn dispatch_out<O>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<O, DispatchError>
where
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    with_ipc_buffer(|ipc_buf| {
        let result = object
            .dispatch(cmd_id)
            .context(ctx)
            .out_size(size_of::<O>())
            .send(ipc_buf)?;
        Ok(*result.value::<O>())
    })
}

pub(crate) fn dispatch_in_out<I, O>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
    input: I,
) -> Result<O, DispatchError>
where
    I: zerocopy::IntoBytes + zerocopy::Immutable,
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    with_ipc_buffer(|ipc_buf| {
        let result = object
            .dispatch(cmd_id)
            .context(ctx)
            .in_raw(input.as_bytes())
            .out_size(size_of::<O>())
            .send(ipc_buf)?;
        Ok(*result.value::<O>())
    })
}

pub(crate) fn dispatch_out_u8(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<u8, DispatchError> {
    dispatch_out::<u8>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_out_bool(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_u8(object, cmd_id, ctx).map(|v| v & 1 != 0)
}

pub(crate) fn dispatch_out_u32(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<u32, DispatchError> {
    dispatch_out::<u32>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_out_i64(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out::<i64>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_in_size_out_buffer(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
    size: i64,
    dst: &mut [u8],
) -> Result<(), DispatchError> {
    with_ipc_buffer(|ipc_buf| {
        object
            .dispatch(cmd_id)
            .context(ctx)
            .in_raw(size.as_bytes())
            .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
            .send(ipc_buf)
            .map(|_| ())
    })
}
