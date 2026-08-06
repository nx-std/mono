//! `IClientProcessMonitor` (sub-object of `ldn:u`/`ldn:s`, `[18.0.0+]`) CMIF
//! dispatch helpers.
//!
//! The interface exposes a single command, `RegisterClient` (cmd 0), which
//! sends the caller's PID via the CMIF special-header `send_pid` flag plus a
//! reserved `u64` payload. The crate is hosversion-unaware — the caller is
//! responsible for only creating the ICPM sub-object on `[18.0.0+]`.

use core::mem::size_of;

use nx_sf::service::{
    DispatchError,
    DomainObjectRef,
};

use crate::proto::CMD_ICPM_REGISTER_CLIENT;

/// `RegisterClient` (cmd 0). Sends `send_pid` + an 8-byte zero payload, per
/// the libnx `_ldnCmdInitialize` helper.
pub(crate) fn register_client(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    let reserved: u64 = 0;
    // SAFETY: `reserved` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const reserved).cast::<u8>(), size_of::<u64>())
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(CMD_ICPM_REGISTER_CLIENT)
        .send_pid()
        .in_raw(in_bytes)
        .send(&mut buf)
        .map(|_| ())
}
