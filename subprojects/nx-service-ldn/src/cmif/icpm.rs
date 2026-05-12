//! `IClientProcessMonitor` (sub-object of `ldn:u`/`ldn:s`, `[18.0.0+]`) CMIF
//! dispatch helpers.
//!
//! The interface exposes a single command, `RegisterClient` (cmd 0), which
//! sends the caller's PID via the CMIF special-header `send_pid` flag plus a
//! reserved `u64` payload. The crate is hosversion-unaware — the caller is
//! responsible for only creating the ICPM sub-object on `[18.0.0+]`.

use core::mem::size_of;

use nx_sf::service::{DispatchError, DomainObject};

use crate::proto::CMD_ICPM_REGISTER_CLIENT;

/// `RegisterClient` (cmd 0). Sends `send_pid` + an 8-byte zero payload, per
/// the libnx `_ldnCmdInitialize` helper.
pub(crate) fn register_client(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    let reserved: u64 = 0;
    // SAFETY: `reserved` lives on the stack until `send()` returns; the
    // dispatcher memcpys the payload before dispatching.
    unsafe {
        object
            .dispatch(CMD_ICPM_REGISTER_CLIENT)
            .send_pid()
            .in_raw((&raw const reserved).cast::<u8>(), size_of::<u64>())
            .send()
            .map(|_| ())
    }
}
