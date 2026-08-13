//! The settings interface's commands, as CMIF requests.

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in_out,
        dispatch_out,
    },
    set::proto::{
        self,
        DeviceNickname,
    },
};

/// Reads the tag of the language the console is set to.
///
/// # Errors
///
/// Returns [`DispatchError`] when the command failed. Nothing is read.
#[inline]
pub(crate) fn get_language_code(session: &Session) -> Result<u64, DispatchError> {
    dispatch_out(session, proto::GET_LANGUAGE_CODE)
}

/// Reads how many language tags the console offers, through `cmd_id`.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_available_language_code_count(
    session: &Session,
    cmd_id: u32,
) -> Result<i32, DispatchError> {
    dispatch_out(session, cmd_id)
}

/// Reads the language tags the console offers into `codes`, and returns how many it wrote.
///
/// The buffer is mapped for the server to write into, which is what the command from `[4.0.0]`
/// expects; [`get_available_language_codes_legacy`] is the same command for the interface before
/// it.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_available_language_codes(
    session: &Session,
    codes: &mut [u64],
) -> Result<i32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_AVAILABLE_LANGUAGE_CODES)
        .out_buffer(codes.as_mut_bytes(), OUT_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}

/// Reads the language tags the console offers into `codes`, and returns how many it wrote.
///
/// The buffer is handed over as a pointer rather than mapped, which is what the interface before
/// `[4.0.0]` expects. That interface also closes the session when asked for more entries than it
/// has, so a caller reads [`get_available_language_code_count`] first and asks for no more.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_available_language_codes_legacy(
    session: &Session,
    codes: &mut [u64],
) -> Result<i32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_AVAILABLE_LANGUAGE_CODES_LEGACY)
        .out_buffer(codes.as_mut_bytes(), OUT_POINTER)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}

/// Reads the tag for the language `index` names.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn language_code_for(session: &Session, index: i32) -> Result<u64, DispatchError> {
    dispatch_in_out(session, proto::MAKE_LANGUAGE_CODE, index)
}

/// Reads which region the console was sold into.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_region_code(session: &Session) -> Result<u32, DispatchError> {
    dispatch_out(session, proto::GET_REGION_CODE)
}

/// Reads whether the console is a retail demo unit.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_quest_flag(session: &Session) -> Result<u8, DispatchError> {
    dispatch_out(session, proto::GET_QUEST_FLAG)
}

/// Reads the name the owner gave the console.
///
/// # Errors
///
/// The same as [`get_language_code`].
#[inline]
pub(crate) fn get_device_nickname(session: &Session) -> Result<DeviceNickname, DispatchError> {
    let mut nickname = DeviceNickname::new();
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    session
        .dispatch(proto::GET_DEVICE_NICKNAME)
        .out_buffer(nickname.as_mut_bytes(), OUT_MAP_ALIAS_FIXED)
        .send(&mut buf)?;

    Ok(nickname)
}

/// A buffer the server maps and writes into.
const OUT_MAP_ALIAS: BufferAttr = BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS);

/// A buffer the server maps and writes into, whose width is part of the command.
const OUT_MAP_ALIAS_FIXED: BufferAttr = OUT_MAP_ALIAS.or(BufferAttr::FIXED_SIZE);

/// A buffer the server writes into through the receive list rather than a mapping.
const OUT_POINTER: BufferAttr = BufferAttr::OUT.or(BufferAttr::HIPC_POINTER);
