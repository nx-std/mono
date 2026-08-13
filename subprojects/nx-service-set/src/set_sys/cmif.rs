//! The system settings interface's commands, as CMIF requests.

use nx_sf::{
    cmif,
    error::{
        ResultCode,
        ToResultCode,
    },
    hipc::OutPointer,
    service::{
        BorrowedSessionHandle,
        BufferAttr,
        DispatchError,
        Session,
    },
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_out,
    set_sys::proto::{
        self,
        FirmwareVersion,
        SettingsItemKey,
        SettingsName,
    },
};

/// Gets the system firmware version using CMIF protocol.
///
/// Uses command ID 4 (GetFirmwareVersion2) which is available on HOS 3.0.0+.
#[inline]
pub(crate) fn get_firmware_version(
    session: BorrowedSessionHandle<'_>,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, proto::GET_FIRMWARE_VERSION_2)
}

/// Gets the system firmware version using CMIF protocol (legacy command).
///
/// Uses command ID 3 (GetFirmwareVersion) for pre-3.0.0 systems.
/// This command zeros the revision field in the output.
#[inline]
pub(crate) fn get_firmware_version_legacy(
    session: BorrowedSessionHandle<'_>,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, proto::GET_FIRMWARE_VERSION)
}

/// Inner implementation that takes a command ID.
fn get_firmware_version_inner(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    // Allocate output buffer on stack.
    let mut out = FirmwareVersion::new();

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .add_out_fixed_pointer(OutPointer::new(out.as_mut_bytes()))
        .build();
    req.send(&mut buf, session)
        .map_err(GetFirmwareVersionError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(GetFirmwareVersionError::ParseResponse)?;

    Ok(out)
}

/// Error returned by [`get_firmware_version`].
#[derive(Debug, thiserror::Error)]
pub enum GetFirmwareVersionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for GetFirmwareVersionError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Reads which of the two system themes is selected.
///
/// # Errors
///
/// Returns [`DispatchError`] when the command failed. Nothing is read.
#[inline]
pub(crate) fn get_color_set_id(session: &Session) -> Result<u32, DispatchError> {
    dispatch_out(session, proto::GET_COLOR_SET_ID)
}

/// Reads how many bytes the item `key` names inside the `name` section takes.
///
/// Both halves of the address are handed over as pointers rather than mapped, each at the full
/// width of its field.
///
/// # Errors
///
/// Returns [`DispatchError`] when the command failed, which is what the interface answers with
/// when it holds no such item. Nothing is read.
#[inline]
pub(crate) fn get_settings_item_value_size(
    session: &Session,
    name: &SettingsName,
    key: &SettingsItemKey,
) -> Result<u64, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_SETTINGS_ITEM_VALUE_SIZE)
        .in_buffer(name.as_bytes(), IN_POINTER)
        .in_buffer(key.as_bytes(), IN_POINTER)
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    Ok(*result.value::<u64>())
}

/// Reads the item `key` names inside the `name` section into `value`, and returns how many bytes
/// the interface wrote.
///
/// The value's buffer is mapped for the server to write into, because an item is as wide as it is
/// and the interface will not carry an arbitrary width through the receive list.
///
/// # Errors
///
/// The same as [`get_settings_item_value_size`].
#[inline]
pub(crate) fn get_settings_item_value(
    session: &Session,
    name: &SettingsName,
    key: &SettingsItemKey,
    value: &mut [u8],
) -> Result<u64, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = session
        .dispatch(proto::GET_SETTINGS_ITEM_VALUE)
        .in_buffer(name.as_bytes(), IN_POINTER)
        .in_buffer(key.as_bytes(), IN_POINTER)
        .out_buffer(value, OUT_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    Ok(*result.value::<u64>())
}

/// A buffer the server reads through the receive list rather than a mapping.
const IN_POINTER: BufferAttr = BufferAttr::IN.or(BufferAttr::HIPC_POINTER);

/// A buffer the server maps and writes into.
const OUT_MAP_ALIAS: BufferAttr = BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS);
