use core::mem::{ManuallyDrop, size_of};

use nx_sf::service::{BufferAttr, DispatchError, DomainObject};

#[inline]
pub(crate) fn as_in_bytes<I: Copy>(input: &I) -> &[u8] {
    unsafe { core::slice::from_raw_parts((&raw const *input).cast::<u8>(), size_of::<I>()) }
}

pub(crate) fn dispatch_no_io(
    object: &ManuallyDrop<DomainObject<'_>>,
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

pub(crate) fn dispatch_in<I: Copy>(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
    input: I,
) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .send(&mut buf)
        .map(|_| ())
}

pub(crate) fn dispatch_out<O: Copy>(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
) -> Result<O, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .out_size(size_of::<O>())
        .send(&mut buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

pub(crate) fn dispatch_in_out<I: Copy, O: Copy>(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
    input: I,
) -> Result<O, DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(as_in_bytes(&input))
        .out_size(size_of::<O>())
        .send(&mut buf)?;
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<O>()) })
}

pub(crate) fn dispatch_out_u8(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
) -> Result<u8, DispatchError> {
    dispatch_out::<u8>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_out_bool(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
) -> Result<bool, DispatchError> {
    dispatch_out_u8(object, cmd_id, ctx).map(|v| v & 1 != 0)
}

pub(crate) fn dispatch_out_u32(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
) -> Result<u32, DispatchError> {
    dispatch_out::<u32>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_out_i64(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
) -> Result<i64, DispatchError> {
    dispatch_out::<i64>(object, cmd_id, ctx)
}

pub(crate) fn dispatch_in_size_out_buffer(
    object: &ManuallyDrop<DomainObject<'_>>,
    cmd_id: u32,
    ctx: u32,
    size: i64,
    dst: &mut [u8],
) -> Result<(), DispatchError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    object
        .dispatch(cmd_id)
        .context(ctx)
        .in_raw(as_in_bytes(&size))
        .out_buffer(dst, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
}
