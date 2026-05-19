//! TIPC protocol operations for set:sys service.
//!
//! This module implements set:sys commands using the TIPC (Trivial IPC) protocol,
//! which is used on HOS 12.0.0+ and by Atmosphere.

use core::mem::size_of;

use nx_sf::{cmif, hipc::BufferMode, tipc};
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
    // Allocate output buffer on stack.
    let mut out = FirmwareVersion::new();

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, cmd_id)
            .add_out_buffer(
                (&raw mut out).cast::<u8>(),
                size_of::<FirmwareVersion>(),
                BufferMode::Normal,
            )
            .send()
            .map_err(GetFirmwareVersionError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetFirmwareVersionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    // Size is 0 because response data comes via buffer, not inline.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(GetFirmwareVersionError::ParseResponse)?;

    Ok(out)
}

/// Error returned by [`get_firmware_version`].
#[derive(Debug, thiserror::Error)]
pub enum GetFirmwareVersionError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
