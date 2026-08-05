use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
};
use zerocopy::IntoBytes as _;

pub(crate) fn dispatch_no_io(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .context(ctx)
        .send(&mut buf)
        .map(|_| ())
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
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(input.as_bytes())
        .send(&mut buf)
        .map(|_| ())
}

pub(crate) fn dispatch_out<O>(
    object: DomainObjectRef<'_>,
    cmd_id: u32,
    ctx: u32,
) -> Result<O, DispatchError>
where
    O: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .out_size(size_of::<O>())
        .send(&mut buf)?;
    Ok(*result.value::<O>())
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
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(input.as_bytes())
        .out_size(size_of::<O>())
        .send(&mut buf)?;
    Ok(*result.value::<O>())
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
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(size.as_bytes())
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}
