//! CMIF protocol operations for set:sys service.
//!
//! This module implements set:sys commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::mem::size_of;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::{CMD_GET_FIRMWARE_VERSION, CMD_GET_FIRMWARE_VERSION_2, FirmwareVersion};

/// Gets the system firmware version using CMIF protocol.
///
/// Uses command ID 4 (GetFirmwareVersion2) which is available on HOS 3.0.0+.
#[inline]
pub fn get_firmware_version(
    session: SessionHandle,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, CMD_GET_FIRMWARE_VERSION_2)
}

/// Gets the system firmware version using CMIF protocol (legacy command).
///
/// Uses command ID 3 (GetFirmwareVersion) for pre-3.0.0 systems.
/// This command zeros the revision field in the output.
#[inline]
pub fn get_firmware_version_legacy(
    session: SessionHandle,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, CMD_GET_FIRMWARE_VERSION)
}

/// Inner implementation that takes a command ID.
fn get_firmware_version_inner(
    session: SessionHandle,
    cmd_id: u32,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // Allocate output buffer on stack
    let mut out = FirmwareVersion::new();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .out_fixed_pointers(1) // One fixed-size output pointer
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Add the output buffer for FirmwareVersion
    // SAFETY: out is valid and properly aligned for FirmwareVersion.
    req.add_out_fixed_pointer((&raw mut out).cast::<u8>(), size_of::<FirmwareVersion>());

    ipc::send_sync_request(session).map_err(GetFirmwareVersionError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetFirmwareVersionError::ParseResponse)?;

    Ok(out)
}

/// Error returned by [`get_firmware_version`].
#[derive(Debug, thiserror::Error)]
pub enum GetFirmwareVersionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
