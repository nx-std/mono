//! CMIF protocol operations for set:sys service.
//!
//! This module implements set:sys commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::{mem::size_of, slice};

use nx_sf::{
    cmif,
    error::{ResultCode, ToResultCode},
    hipc::OutPointer,
    ipc::Handle as SessionHandle,
};

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
    // Allocate output buffer on stack.
    let mut out = FirmwareVersion::new();

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `FirmwareVersion` is `#[repr(C)]` with only `u8`/byte-array fields,
    // so it is plain-old-data and any byte pattern is a valid value. The borrow
    // is exclusive (we hold `&mut out`) and covers the full size of the struct.
    let out_bytes: &mut [u8] = unsafe {
        slice::from_raw_parts_mut((&raw mut out).cast::<u8>(), size_of::<FirmwareVersion>())
    };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .add_out_fixed_pointer(OutPointer::new(out_bytes))
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
