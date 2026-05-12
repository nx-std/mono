//! CMIF protocol operations for the fsp-pr service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain};

use crate::proto;

/// Input structure for the `RegisterProgram` command.
#[repr(C)]
struct RegisterProgramIn {
    storage_id: u8,
    fs_access_control_restriction_mode: u8,
    _pad: [u8; 6],
    pid: u64,
    tid: u64,
    fah_size: u64,
    fac_size: u64,
}

/// Registers a program's filesystem access controls (cmd 0).
#[allow(clippy::too_many_arguments)]
pub fn register_program(
    domain: &Domain,
    pid: u64,
    tid: u64,
    storage_id: u8,
    fs_access_header: &[u8],
    fs_access_control: &[u8],
    fs_access_control_restriction_mode: u8,
) -> Result<(), DispatchError> {
    let input = RegisterProgramIn {
        storage_id,
        fs_access_control_restriction_mode,
        _pad: [0; 6],
        pid,
        tid,
        fah_size: fs_access_header.len() as u64,
        fac_size: fs_access_control.len() as u64,
    };

    // SAFETY: `input` lives on the stack until `send()` returns.
    unsafe {
        domain
            .dispatch(proto::REGISTER_PROGRAM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<RegisterProgramIn>(),
            )
            .buffer(
                fs_access_header.as_ptr(),
                fs_access_header.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .buffer(
                fs_access_control.as_ptr(),
                fs_access_control.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// Unregisters a program (cmd 1).
pub fn unregister_program(domain: &Domain, pid: u64) -> Result<(), DispatchError> {
    // SAFETY: `pid` lives on the stack until `send()` returns.
    unsafe {
        domain
            .dispatch(proto::UNREGISTER_PROGRAM)
            .in_raw((&raw const pid).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}

/// Sets the current process on the fsp-pr session (cmd 2, `[4.0.0+]`).
pub fn set_current_process(domain: &Domain) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    // SAFETY: `pid_placeholder` lives on the stack until `send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_CURRENT_PROCESS)
            .send_pid()
            .in_raw((&raw const pid_placeholder).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}

/// Enables or disables program verification (cmd 256, pre-`[10.0.0]`).
pub fn set_enabled_program_verification(
    domain: &Domain,
    enabled: bool,
) -> Result<(), DispatchError> {
    let value: u8 = u8::from(enabled);
    // SAFETY: `value` lives on the stack until `send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_ENABLED_PROGRAM_VERIFICATION)
            .in_raw((&raw const value).cast::<u8>(), size_of::<u8>())
            .send()
            .map(|_| ())
    }
}
