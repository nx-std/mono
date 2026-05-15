//! ns:dev CMIF commands.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out},
    proto,
    types::{
        LaunchProperties, NsdevLaunchApplicationForDevelopIn,
        NsdevLaunchApplicationWithStorageIdIn, NsdevLaunchProgramIn, ShellEventInfo,
    },
};

/// LaunchProgram (cmd 0).
#[inline]
pub(crate) fn launch_program(
    service: &Session,
    input: NsdevLaunchProgramIn,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::NSDEV_LAUNCH_PROGRAM, input)
}

/// TerminateProcess (cmd 1).
#[inline]
pub(crate) fn terminate_process(service: &Session, pid: u64) -> Result<(), DispatchError> {
    dispatch_in(service, proto::NSDEV_TERMINATE_PROCESS, pid)
}

/// TerminateProgram (cmd 2).
#[inline]
pub(crate) fn terminate_program(service: &Session, tid: u64) -> Result<(), DispatchError> {
    dispatch_in(service, proto::NSDEV_TERMINATE_PROGRAM, tid)
}

/// GetShellEvent (cmd 4) — returns copy handle.
pub(crate) fn get_shell_event(service: &Session) -> Result<u32, AcquireEventError> {
    let result = service
        .dispatch(proto::NSDEV_GET_SHELL_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(AcquireEventError::Dispatch)?;

    let Some(handle) = result.copy_handles.first().copied() else {
        return Err(AcquireEventError::MissingHandle);
    };

    Ok(handle)
}

/// GetShellEventInfo (cmd 5).
#[inline]
pub(crate) fn get_shell_event_info(service: &Session) -> Result<ShellEventInfo, DispatchError> {
    dispatch_out(service, proto::NSDEV_GET_SHELL_EVENT_INFO)
}

/// TerminateApplication (cmd 6).
#[inline]
pub(crate) fn terminate_application(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::NSDEV_TERMINATE_APPLICATION)
}

/// PrepareLaunchProgramFromHost (cmd 7).
pub(crate) fn prepare_launch_program_from_host(
    service: &Session,
    path: &[u8],
) -> Result<LaunchProperties, DispatchError> {
    let result = service
        .dispatch(proto::NSDEV_PREPARE_LAUNCH_PROGRAM_FROM_HOST)
        .out_size(size_of::<LaunchProperties>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<LaunchProperties>()) })
}

/// LaunchApplicationForDevelop (cmd 8).
#[inline]
pub(crate) fn launch_application_for_develop(
    service: &Session,
    input: NsdevLaunchApplicationForDevelopIn,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::NSDEV_LAUNCH_APPLICATION_FOR_DEVELOP, input)
}

/// LaunchApplicationFromHost (cmd 8) — uses u32 flags + MapAlias-In path.
pub(crate) fn launch_application_from_host(
    service: &Session,
    flags: u32,
    path: &[u8],
) -> Result<u64, DispatchError> {
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const flags).cast::<u8>(), size_of::<u32>()) };
    let result = service
        .dispatch(proto::NSDEV_LAUNCH_APPLICATION_FROM_HOST)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .in_buffer(path, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// LaunchApplicationWithStorageIdForDevelop (cmd 9).
#[inline]
pub(crate) fn launch_application_with_storage_id_for_develop(
    service: &Session,
    input: NsdevLaunchApplicationWithStorageIdIn,
) -> Result<u64, DispatchError> {
    dispatch_in_out(
        service,
        proto::NSDEV_LAUNCH_APPLICATION_WITH_STORAGE_ID_FOR_DEVELOP,
        input,
    )
}

/// IsSystemMemoryResourceLimitBoosted (cmd 10).
#[inline]
pub(crate) fn is_system_memory_resource_limit_boosted(
    service: &Session,
) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(
        service,
        proto::NSDEV_IS_SYSTEM_MEMORY_RESOURCE_LIMIT_BOOSTED,
    )?;
    Ok(val & 1 != 0)
}

/// GetRunningApplicationProcessIdForDevelop (cmd 11).
#[inline]
pub(crate) fn get_running_application_process_id_for_develop(
    service: &Session,
) -> Result<u64, DispatchError> {
    dispatch_out(
        service,
        proto::NSDEV_GET_RUNNING_APPLICATION_PROCESS_ID_FOR_DEVELOP,
    )
}

/// SetCurrentApplicationRightsEnvironmentCanBeActive (cmd 12).
#[inline]
pub(crate) fn set_current_application_rights_environment_can_be_active(
    service: &Session,
    flag: u8,
) -> Result<(), DispatchError> {
    dispatch_in(
        service,
        proto::NSDEV_SET_CURRENT_APPLICATION_RIGHTS_ENVIRONMENT_CAN_BE_ACTIVE,
        flag,
    )
}

/// Error returned by event acquisition commands.
#[derive(Debug, thiserror::Error)]
pub enum AcquireEventError {
    #[error("failed to dispatch event acquisition")]
    Dispatch(#[source] DispatchError),
    #[error("event acquisition response did not include expected copy handle")]
    MissingHandle,
}
