//! IReadOnlyApplicationControlDataInterface CMIF commands.

use core::{
    mem::size_of,
    ptr,
};

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};

use super::app_manager::AsyncCommandError;
use crate::{
    proto,
    types::{
        ControlData2In,
        ControlDataSourceAppIdIn,
        ListApplicationTitleIn,
    },
};

/// GetApplicationControlData (cmd 0).
pub(crate) fn get_application_control_data(
    service: &Session,
    input: ControlDataSourceAppIdIn,
    out: &mut [u8],
) -> Result<u32, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ControlDataSourceAppIdIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CTRL_DATA_GET_APPLICATION_CONTROL_DATA)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// GetApplicationDesiredLanguage (cmd 1).
pub(crate) fn get_application_desired_language(
    service: &Session,
    lang_bitmask: u8,
) -> Result<u8, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const lang_bitmask).cast::<u8>(), size_of::<u8>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CTRL_DATA_GET_APPLICATION_DESIRED_LANGUAGE)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) })
}

/// GetApplicationControlData2 (cmd 6).
pub(crate) fn get_application_control_data2(
    service: &Session,
    input: ControlData2In,
    out: &mut [u8],
) -> Result<u64, DispatchError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<ControlData2In>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CTRL_DATA_GET_APPLICATION_CONTROL_DATA2)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .out_buffer(out, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// ListApplicationTitle2 (cmd 10) — tmem-based async command.
pub(crate) fn list_application_title2(
    service: &Session,
    input: ListApplicationTitleIn,
    tmem_handle: u32,
    app_ids: &[u8],
) -> Result<super::app_manager::AsyncOut, AsyncCommandError> {
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ListApplicationTitleIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::CTRL_DATA_LIST_APPLICATION_TITLE2)
        .in_raw(in_bytes)
        .in_buffer(app_ids, BufferAttr::HIPC_MAP_ALIAS)
        .in_handle(tmem_handle)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(AsyncCommandError::Dispatch)?;

    super::app_manager::extract_async_out(&result)
}
