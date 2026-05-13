//! CMIF protocol operations for the filesystem loader service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain};

use crate::{
    dispatch::dispatch_in_out,
    proto,
    types::{
        FS_MAX_PATH, FsCodeInfo, OpenCodeFileSystemAttrIn, OpenCodeFileSystemTidIn,
        OpenCodeFileSystemV20In, SetCurrentProcessIn,
    },
};

/// Opens a code filesystem (pre-10.0.0).
///
/// Takes a title ID and a path buffer. Does not return code info.
/// Returns the domain sub-object ID for the opened filesystem.
pub(crate) fn open_code_filesystem_legacy(
    domain: &Domain,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
) -> Result<u32, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemTidIn { tid };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<OpenCodeFileSystemTidIn>(),
            )
            .buffer(
                path.as_ptr(),
                FS_MAX_PATH,
                BufferAttr::IN
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .out_objects(1)
            .send()
            .map_err(OpenCodeFileSystemError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(OpenCodeFileSystemError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Opens a code filesystem (10.0.0–15.x).
///
/// Takes a title ID and a path buffer. Returns code info and the domain
/// sub-object ID for the opened filesystem.
pub(crate) fn open_code_filesystem_v10(
    domain: &Domain,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<u32, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemTidIn { tid };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<OpenCodeFileSystemTidIn>(),
            )
            .buffer(
                (out_code_info as *mut FsCodeInfo).cast::<u8>(),
                size_of::<FsCodeInfo>(),
                BufferAttr::OUT
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .buffer(
                path.as_ptr(),
                FS_MAX_PATH,
                BufferAttr::IN
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .out_objects(1)
            .send()
            .map_err(OpenCodeFileSystemError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(OpenCodeFileSystemError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Opens a code filesystem (16.0.0–16.x).
///
/// Takes content attributes, title ID, and a path buffer. Returns code info
/// via HIPC pointer and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v16(
    domain: &Domain,
    content_attributes: u8,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<u32, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemAttrIn::new(content_attributes, tid);

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<OpenCodeFileSystemAttrIn>(),
            )
            .buffer(
                (out_code_info as *mut FsCodeInfo).cast::<u8>(),
                size_of::<FsCodeInfo>(),
                BufferAttr::OUT
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .buffer(
                path.as_ptr(),
                FS_MAX_PATH,
                BufferAttr::IN
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .out_objects(1)
            .send()
            .map_err(OpenCodeFileSystemError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(OpenCodeFileSystemError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Opens a code filesystem (17.0.0–19.x).
///
/// Takes content attributes, title ID, and a path buffer. Returns code info
/// via HIPC map-alias and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v17(
    domain: &Domain,
    content_attributes: u8,
    tid: u64,
    path: &[u8; FS_MAX_PATH],
    out_code_info: &mut FsCodeInfo,
) -> Result<u32, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemAttrIn::new(content_attributes, tid);

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<OpenCodeFileSystemAttrIn>(),
            )
            .buffer(
                path.as_ptr(),
                FS_MAX_PATH,
                BufferAttr::IN
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::FIXED_SIZE),
            )
            .buffer(
                (out_code_info as *mut FsCodeInfo).cast::<u8>(),
                size_of::<FsCodeInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .out_objects(1)
            .send()
            .map_err(OpenCodeFileSystemError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(OpenCodeFileSystemError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Opens a code filesystem (20.0.0+).
///
/// Takes content attributes, storage ID, and title ID. No path buffer.
/// Returns code info via HIPC map-alias and the domain sub-object ID.
pub(crate) fn open_code_filesystem_v20(
    domain: &Domain,
    content_attributes: u8,
    storage_id: u8,
    tid: u64,
    out_code_info: &mut FsCodeInfo,
) -> Result<u32, OpenCodeFileSystemError> {
    let input = OpenCodeFileSystemV20In::new(content_attributes, storage_id, tid);

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        domain
            .dispatch(proto::OPEN_CODE_FILE_SYSTEM)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<OpenCodeFileSystemV20In>(),
            )
            .buffer(
                (out_code_info as *mut FsCodeInfo).cast::<u8>(),
                size_of::<FsCodeInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .out_objects(1)
            .send()
            .map_err(OpenCodeFileSystemError::Dispatch)?
    };

    if result.objects.is_empty() {
        return Err(OpenCodeFileSystemError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Checks whether a program (by PID) is archived.
pub(crate) fn is_archived_program(domain: &Domain, pid: u64) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(domain, proto::IS_ARCHIVED_PROGRAM, pid)?;
    Ok(val & 1 != 0)
}

/// Sets the current process on the service session.
pub(crate) fn set_current_process(domain: &Domain) -> Result<(), DispatchError> {
    let input = SetCurrentProcessIn { pid_placeholder: 0 };

    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        domain
            .dispatch(proto::SET_CURRENT_PROCESS)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<SetCurrentProcessIn>(),
            )
            .send_pid()
            .send()
            .map(|_| ())
    }
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
