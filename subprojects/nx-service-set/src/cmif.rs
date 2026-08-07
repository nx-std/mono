//! CMIF protocol operations for set:sys service.
//!
//! This module implements set:sys commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_sf::{
    cmif,
    error::{
        ResultCode,
        ToResultCode,
    },
    hipc::OutPointer,
    service::BorrowedSessionHandle,
};
use zerocopy::IntoBytes as _;

use crate::proto::{
    CMD_GET_FIRMWARE_VERSION,
    CMD_GET_FIRMWARE_VERSION_2,
    FirmwareVersion,
};

/// Gets the system firmware version using CMIF protocol.
///
/// Uses command ID 4 (GetFirmwareVersion2) which is available on HOS 3.0.0+.
#[inline]
pub fn get_firmware_version(
    session: BorrowedSessionHandle<'_>,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, CMD_GET_FIRMWARE_VERSION_2)
}

/// Gets the system firmware version using CMIF protocol (legacy command).
///
/// Uses command ID 3 (GetFirmwareVersion) for pre-3.0.0 systems.
/// This command zeros the revision field in the output.
#[inline]
pub fn get_firmware_version_legacy(
    session: BorrowedSessionHandle<'_>,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, CMD_GET_FIRMWARE_VERSION)
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
