//! `ILibraryAppletCreator` commands.
//!
//! The creator mints the two kinds of object a launch needs: one accessor that
//! drives a library applet, and the storages that carry data across to it.

use nx_sf::{
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    service::{
        DispatchError,
        DomainObjectRef,
    },
};
use nx_svc::error::ResultCode;
use zerocopy::IntoBytes as _;

use super::{
    accessor::LibraryAppletAccessor,
    storage::Storage,
    support::reanchor_object,
};
use crate::proto::{
    AppletId,
    CMD_LAC_CREATE_LIBRARY_APPLET,
    CMD_LAC_CREATE_STORAGE,
    LibraryAppletMode,
};

/// Creates a library applet, returning the accessor that drives it (cmd 0).
///
/// The applet is created but not started; see
/// [`LibraryAppletAccessor::start`]. Callers must have waited on the launchable
/// event first - see the [module docs](super).
pub fn create_library_applet<'d>(
    creator: DomainObjectRef<'d>,
    applet_id: AppletId,
    mode: LibraryAppletMode,
) -> Result<LibraryAppletAccessor<'d>, CreateLibraryAppletError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let request = CreateLibraryAppletIn {
        applet_id: applet_id.as_raw(),
        mode: mode.as_raw(),
    };

    let mut result = creator
        .dispatch(CMD_LAC_CREATE_LIBRARY_APPLET)
        .in_raw(request.as_bytes())
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateLibraryAppletError::Dispatch)?;

    let raw_object_id = result
        .take_object(0)
        .ok_or(CreateLibraryAppletError::MissingObject)?
        .into_raw_object_id();

    Ok(LibraryAppletAccessor::new(reanchor_object(
        creator.domain(),
        raw_object_id,
    )))
}

/// Request payload for `CreateLibraryApplet`.
#[derive(Debug, Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C)]
struct CreateLibraryAppletIn {
    applet_id: u32,
    mode: u32,
}

/// Error returned by [`create_library_applet`].
#[derive(Debug, thiserror::Error)]
pub enum CreateLibraryAppletError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not carry the accessor object.
    #[error("missing accessor object in response")]
    MissingObject,
}

impl ToResultCode for CreateLibraryAppletError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingObject => GENERIC_ERROR,
        }
    }
}

/// Creates an `IStorage` of `size` bytes (cmd 10).
///
/// The storage is allocated server-side and starts zeroed; write into it through
/// [`Storage::write`] before pushing it to an applet.
pub fn create_storage<'d>(
    creator: DomainObjectRef<'d>,
    size: i64,
) -> Result<Storage<'d>, CreateStorageError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = creator
        .dispatch(CMD_LAC_CREATE_STORAGE)
        .in_raw(size.as_bytes())
        .out_objects(1)
        .send(&mut buf)
        .map_err(CreateStorageError::Dispatch)?;

    let raw_object_id = result
        .take_object(0)
        .ok_or(CreateStorageError::MissingObject)?
        .into_raw_object_id();

    Ok(Storage::new(reanchor_object(
        creator.domain(),
        raw_object_id,
    )))
}

/// Error returned by [`create_storage`].
#[derive(Debug, thiserror::Error)]
pub enum CreateStorageError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not carry the storage object.
    #[error("missing storage object in response")]
    MissingObject,
}

impl ToResultCode for CreateStorageError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingObject => GENERIC_ERROR,
        }
    }
}
