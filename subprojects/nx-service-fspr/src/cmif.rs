//! CMIF protocol operations for the fsp-pr service.

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Domain,
};
use zerocopy::IntoBytes as _;

use crate::proto;

/// Input structure for the `RegisterProgram` command.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::REGISTER_PROGRAM)
        .in_raw(input.as_bytes())
        .in_buffer(fs_access_header, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(fs_access_control, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Unregisters a program (cmd 1).
pub fn unregister_program(domain: &Domain, pid: u64) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::UNREGISTER_PROGRAM)
        .in_raw(pid.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets the current process on the fsp-pr session (cmd 2, `[4.0.0+]`).
pub fn set_current_process(domain: &Domain) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_CURRENT_PROCESS)
        .send_pid()
        .in_raw(pid_placeholder.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Enables or disables program verification (cmd 256, pre-`[10.0.0]`).
pub fn set_enabled_program_verification(
    domain: &Domain,
    enabled: bool,
) -> Result<(), DispatchError> {
    let value: u8 = u8::from(enabled);
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_ENABLED_PROGRAM_VERIFICATION)
        .in_raw(value.as_bytes())
        .send(&mut ipc_buf)
        .map(|_| ())
}
