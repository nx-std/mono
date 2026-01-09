//! TIPC protocol operations for set:sys service.
//!
//! This module implements set:sys commands using the TIPC (Trivial IPC) protocol,
//! which is used on HOS 12.0.0+ and by Atmosphere.

use core::mem::size_of;

use nx_sf::{hipc::BufferMode, tipc};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::{CMD_GET_FIRMWARE_VERSION, CMD_GET_FIRMWARE_VERSION_2, FirmwareVersion};

/// Gets the system firmware version using TIPC protocol.
///
/// Uses command ID 4 (GetFirmwareVersion2).
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn get_firmware_version(
    session: SessionHandle,
) -> Result<FirmwareVersion, GetFirmwareVersionError> {
    get_firmware_version_inner(session, CMD_GET_FIRMWARE_VERSION_2)
}

/// Gets the system firmware version using TIPC protocol (legacy command).
///
/// Uses command ID 3 (GetFirmwareVersion).
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

    let fmt = tipc::RequestFormat {
        request_id: cmd_id,
        data_size: 0, // No input data
        num_in_buffers: 0,
        num_out_buffers: 1, // One output buffer for FirmwareVersion
        num_inout_buffers: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { tipc::make_request(ipc_buf, fmt) };

    // Add the output buffer for FirmwareVersion
    // SAFETY: out is valid and properly aligned for FirmwareVersion.
    req.add_out_buffer(
        (&raw mut out).cast::<u8>(),
        size_of::<FirmwareVersion>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetFirmwareVersionError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    // Size is 0 because response data comes via buffer, not inline.
    let _resp = unsafe { tipc::parse_response(ipc_buf, 0) }
        .map_err(GetFirmwareVersionError::ParseResponse)?;

    Ok(out)
}

/// Error returned by [`get_firmware_version`].
#[derive(Debug, thiserror::Error)]
pub enum GetFirmwareVersionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
