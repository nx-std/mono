//! CMIF protocol operations for the Mii service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};

use crate::{
    dispatch::{
        dispatch_in_out,
        dispatch_out,
    },
    proto,
    types::{
        BuildRandomIn,
        MiiCharInfo,
        MiiSourceFlag,
    },
};

/// Opens a Mii database sub-object on the root service.
///
/// Returns the database session handle (move handle).
pub(crate) fn open_database(service: &Session, key_code: u32) -> Result<u32, OpenDatabaseError> {
    // SAFETY: `key_code` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const key_code).cast::<u8>(), size_of::<u32>())
    };
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::OPEN_DATABASE)
        .in_raw(in_bytes)
        .send(&mut buf)
        .map_err(OpenDatabaseError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenDatabaseError::MissingHandle);
    }
    Ok(result.move_handles[0])
}

/// Checks if the database has been updated.
pub(crate) fn db_is_updated(service: &Session, flag: MiiSourceFlag) -> Result<bool, DispatchError> {
    let raw: u8 = dispatch_in_out(service, proto::DB_IS_UPDATED, flag.bits())?;
    Ok(raw & 1 != 0)
}

/// Checks if the database is full.
pub(crate) fn db_is_full(service: &Session) -> Result<bool, DispatchError> {
    let raw: u8 = dispatch_out(service, proto::DB_IS_FULL)?;
    Ok(raw & 1 != 0)
}

/// Gets the number of Miis matching a source flag.
pub(crate) fn db_get_count(service: &Session, flag: MiiSourceFlag) -> Result<i32, DispatchError> {
    dispatch_in_out(service, proto::DB_GET_COUNT, flag.bits())
}

/// Gets Mii character info entries matching a source flag.
///
/// Returns the number of entries actually written.
pub(crate) fn db_get1(
    service: &Session,
    flag: MiiSourceFlag,
    buffer: &mut [MiiCharInfo],
) -> Result<i32, DispatchError> {
    let flag_val = flag.bits();

    // SAFETY: `flag_val` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const flag_val).cast::<u8>(), size_of::<u32>())
    };
    // SAFETY: `buffer` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `buffer`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            buffer.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(buffer),
        )
    };
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::DB_GET1)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)?;

    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Builds a random Mii character info.
pub(crate) fn db_build_random(
    service: &Session,
    age: u32,
    gender: u32,
    face_color: u32,
) -> Result<MiiCharInfo, DispatchError> {
    dispatch_in_out(
        service,
        proto::DB_BUILD_RANDOM,
        BuildRandomIn {
            age,
            gender,
            face_color,
        },
    )
}

/// Error returned by [`open_database`].
#[derive(Debug, thiserror::Error)]
pub enum OpenDatabaseError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenDatabase")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenDatabase response missing move handle")]
    MissingHandle,
}
