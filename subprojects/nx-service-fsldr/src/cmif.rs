//! CMIF protocol operations for the filesystem loader service.

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Domain,
    DomainObject,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_in_out,
    proto,
    types::{
        FS_MAX_PATH,
        FsCodeInfo,
        OpenCodeFileSystemAttrIn,
        OpenCodeFileSystemTidIn,
        OpenCodeFileSystemV20In,
        SetCurrentProcessIn,
    },
};

/// Opens a code filesystem (pre-10.0.0).
///
/// Takes a title ID and a path buffer. Does not return code info.
pub(crate) fn open_code_filesystem_legacy<'d>(
    domain: &'d Domain,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
) -> Result<DomainObject<'d>, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemTidIn { tid };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
        .in_raw(input.as_bytes())
        .in_buffer(
            path.as_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(OpenCodeFileSystemError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(OpenCodeFileSystemError::MissingObject)
}

/// Opens a code filesystem (10.0.0–15.x).
///
/// Takes a title ID and a path buffer. Returns code info via HIPC pointer.
pub(crate) fn open_code_filesystem_v10<'d>(
    domain: &'d Domain,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<DomainObject<'d>, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemTidIn { tid };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
        .in_raw(input.as_bytes())
        .out_buffer(
            out_code_info.as_mut_bytes(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .in_buffer(
            path.as_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(OpenCodeFileSystemError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(OpenCodeFileSystemError::MissingObject)
}

/// Opens a code filesystem (16.0.0–16.x).
///
/// Takes content attributes, title ID, and a path buffer. Returns code info
/// via HIPC pointer and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v16<'d>(
    domain: &'d Domain,
    content_attributes: u8,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<DomainObject<'d>, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemAttrIn::new(content_attributes, tid);

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
        .in_raw(input.as_bytes())
        .out_buffer(
            out_code_info.as_mut_bytes(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .in_buffer(
            path.as_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(OpenCodeFileSystemError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(OpenCodeFileSystemError::MissingObject)
}

/// Opens a code filesystem (17.0.0–19.x).
///
/// Takes content attributes, title ID, and a path buffer. Returns code info
/// via HIPC map-alias and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v17<'d>(
    domain: &'d Domain,
    content_attributes: u8,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<DomainObject<'d>, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemAttrIn::new(content_attributes, tid);

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
        .in_raw(input.as_bytes())
        .in_buffer(
            path.as_slice(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .out_buffer(out_code_info.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(OpenCodeFileSystemError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(OpenCodeFileSystemError::MissingObject)
}

/// Opens a code filesystem (20.0.0+).
///
/// Takes content attributes, storage ID, and title ID. No path buffer.
/// Returns code info via HIPC map-alias and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v20<'d>(
    domain: &'d Domain,
    content_attributes: u8,
    storage_id: u8,
    tid: u64,
    out_code_info: &mut FsCodeInfo,
) -> Result<DomainObject<'d>, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemV20In::new(content_attributes, storage_id, tid);

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
        .in_raw(input.as_bytes())
        .out_buffer(out_code_info.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(OpenCodeFileSystemError::Dispatch)?;

    result
        .take_object(0)
        .ok_or(OpenCodeFileSystemError::MissingObject)
}

/// Checks whether a program (by PID) is archived.
pub(crate) fn is_archived_program(domain: &Domain, pid: u64) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(domain, proto::IS_ARCHIVED_PROGRAM, pid)?;
    Ok(val & 1 != 0)
}

/// Sets the current process on the service session.
pub(crate) fn set_current_process(domain: &Domain) -> Result<(), DispatchError> {
    let input = SetCurrentProcessIn { pid_placeholder: 0 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    domain
        .dispatch(proto::SET_CURRENT_PROCESS)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Error returned by `open_code_filesystem_*` operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenCodeFileSystemError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenCodeFileSystem")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected sub-object id.
    #[error("OpenCodeFileSystem response did not include the expected sub-object")]
    MissingObject,
}
