//! CMIF protocol operations for the RO service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_in_pid,
    proto,
    types::{
        LoadNroIn,
        LoadNrrIn,
        LoaderModuleInfo,
        UnloadIn,
    },
};

/// Initializes the RO service session by registering the current process.
///
/// Sends the current PID and `CUR_PROCESS_HANDLE` as a copy handle.
pub(crate) fn initialize(service: &Session) -> Result<(), DispatchError> {
    let pid_placeholder: u64 = 0;
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::INITIALIZE)
        .in_raw(pid_placeholder.as_bytes())
        .send_pid()
        .in_handle(nx_svc::raw::CUR_PROCESS_HANDLE)
        .send(&mut buf)
        .map(|_| ())
}

/// Loads an NRO module into the process address space.
///
/// Returns the load address on success.
pub(crate) fn load_nro(
    service: &Session,
    nro_address: u64,
    nro_size: u64,
    bss_address: u64,
    bss_size: u64,
) -> Result<u64, DispatchError> {
    let input = LoadNroIn {
        pid_placeholder: 0,
        nro_address,
        nro_size,
        bss_address,
        bss_size,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::LOAD_NRO)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_size(size_of::<u64>())
        .send(&mut buf)?;

    Ok(*result.value::<u64>())
}

/// Unloads a previously loaded NRO module.
pub(crate) fn unload_nro(service: &Session, nro_address: u64) -> Result<(), DispatchError> {
    let input = UnloadIn {
        pid_placeholder: 0,
        address: nro_address,
    };
    dispatch_in_pid(service, proto::UNLOAD_NRO, input)
}

/// Loads an NRR (NRO registration record).
pub(crate) fn load_nrr(
    service: &Session,
    nrr_address: u64,
    nrr_size: u64,
) -> Result<(), DispatchError> {
    let input = LoadNrrIn {
        pid_placeholder: 0,
        nrr_address,
        nrr_size,
    };
    dispatch_in_pid(service, proto::LOAD_NRR, input)
}

/// Unloads a previously loaded NRR.
pub(crate) fn unload_nrr(service: &Session, nrr_address: u64) -> Result<(), DispatchError> {
    let input = UnloadIn {
        pid_placeholder: 0,
        address: nrr_address,
    };
    dispatch_in_pid(service, proto::UNLOAD_NRR, input)
}

/// Loads an NRR with extended validation (`[7.0.0+]`).
pub(crate) fn load_nrr_ex(
    service: &Session,
    nrr_address: u64,
    nrr_size: u64,
) -> Result<(), DispatchError> {
    let input = LoadNrrIn {
        pid_placeholder: 0,
        nrr_address,
        nrr_size,
    };
    dispatch_in_pid(service, proto::LOAD_NRR_EX, input)
}

/// Gets module information for a process via `ro:dmnt`.
///
/// Writes module info entries into `out_modules` and returns the number
/// of entries written by the service.
pub(crate) fn get_process_module_info(
    service: &Session,
    pid: u64,
    out_modules: &mut [LoaderModuleInfo],
) -> Result<i32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_PROCESS_MODULE_INFO)
        .in_raw(pid.as_bytes())
        .out_buffer(out_modules.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut buf)?;

    Ok(*result.value::<i32>())
}
