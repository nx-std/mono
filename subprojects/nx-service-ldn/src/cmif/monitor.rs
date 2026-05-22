//! `IMonitorService` (sub-object of `ldn:m`) CMIF dispatch helpers.
//!
//! Mirrors libnx's `ldnm*` calls. The crate is hosversion-unaware; every
//! command is exposed regardless of HOS version.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_no_io, dispatch_out},
    proto::{
        CMD_MON_FINALIZE, CMD_MON_GET_IPV4_ADDRESS, CMD_MON_GET_NETWORK_CONFIG,
        CMD_MON_GET_NETWORK_INFO, CMD_MON_GET_SECURITY_PARAMETER, CMD_MON_GET_STATE,
        CMD_MON_INITIALIZE, LdnState,
    },
    types::{
        LdnIpv4Address, LdnNetworkConfig, LdnNetworkInfo, LdnSecurityParameter, LdnSubnetMask,
    },
};

/// `InitializeMonitor` (cmd 100).
pub(crate) fn initialize_monitor(session: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(session, CMD_MON_INITIALIZE)
}

/// `FinalizeMonitor` (cmd 101).
pub(crate) fn finalize_monitor(session: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(session, CMD_MON_FINALIZE)
}

/// `GetState` (cmd 0). Returns the raw u32 alongside a parsed [`LdnState`].
pub(crate) fn get_state(session: &Session) -> Result<LdnState, GetStateError> {
    let raw = dispatch_out::<u32>(session, CMD_MON_GET_STATE).map_err(GetStateError::Dispatch)?;
    LdnState::from_raw(raw).ok_or(GetStateError::InvalidState(raw))
}

/// Error returned by [`get_state`].
#[derive(Debug, thiserror::Error)]
pub enum GetStateError {
    #[error("failed to dispatch GetState")]
    Dispatch(#[source] DispatchError),
    #[error("invalid LdnState: {0}")]
    InvalidState(u32),
}

/// `GetNetworkInfo` (cmd 1). Fills the caller-supplied output buffer.
pub(crate) fn get_network_info(
    session: &Session,
    out: &mut LdnNetworkInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `out` is a valid `&mut LdnNetworkInfo`; viewing it as bytes for
    // the OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut LdnNetworkInfo).cast::<u8>(),
            size_of::<LdnNetworkInfo>(),
        )
    };
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    session
        .dispatch(CMD_MON_GET_NETWORK_INFO)
        .out_buffer(
            out_bytes,
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut buf)
        .map(|_| ())
}

/// `GetIpv4Address` (cmd 2). Returns `(addr, mask)`.
pub(crate) fn get_ipv4_address(
    session: &Session,
) -> Result<(LdnIpv4Address, LdnSubnetMask), DispatchError> {
    #[derive(Clone, Copy)]
    #[repr(C)]
    struct Out {
        addr: LdnIpv4Address,
        mask: LdnSubnetMask,
    }
    let out = dispatch_out::<Out>(session, CMD_MON_GET_IPV4_ADDRESS)?;
    Ok((out.addr, out.mask))
}

/// `GetSecurityParameter` (cmd 4).
pub(crate) fn get_security_parameter(
    session: &Session,
) -> Result<LdnSecurityParameter, DispatchError> {
    dispatch_out::<LdnSecurityParameter>(session, CMD_MON_GET_SECURITY_PARAMETER)
}

/// `GetNetworkConfig` (cmd 5).
pub(crate) fn get_network_config(session: &Session) -> Result<LdnNetworkConfig, DispatchError> {
    dispatch_out::<LdnNetworkConfig>(session, CMD_MON_GET_NETWORK_CONFIG)
}
