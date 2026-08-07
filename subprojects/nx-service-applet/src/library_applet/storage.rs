//! `IStorage` and `IStorageAccessor` commands.
//!
//! A storage is a fixed-size server-side buffer, and it is the only way data
//! crosses between this process and a library applet. Reading or writing one
//! goes through a second object: `IStorage` holds the bytes, `IStorageAccessor`
//! is what actually moves them.

use nx_sf::{
    cmif::ObjectId,
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    service::{
        BufferAttr,
        DispatchError,
        DomainObject,
    },
};
use nx_svc::error::ResultCode;
use zerocopy::IntoBytes as _;

use super::support::reanchor_object;
use crate::proto::{
    CMD_STORAGE_ACCESSOR_GET_SIZE,
    CMD_STORAGE_ACCESSOR_READ,
    CMD_STORAGE_ACCESSOR_WRITE,
    CMD_STORAGE_OPEN,
};

/// An `IStorage` created by [`create_storage`](super::create_storage).
///
/// Closes the server-side object on drop. Pushing a storage to an applet does
/// not consume it: the server copies the contents, so the storage is still ours
/// to close afterwards.
#[derive(Debug)]
pub struct Storage<'d> {
    object: DomainObject<'d>,
}

impl<'d> Storage<'d> {
    /// Wraps the domain object this storage is addressed through.
    pub(super) fn new(object: DomainObject<'d>) -> Self {
        Self { object }
    }

    /// Returns the object id, for passing this storage as a command input.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object.object_id()
    }

    /// Opens an accessor onto this storage (cmd 0).
    ///
    /// Each accessor is a distinct server-side object; the convenience methods
    /// below open one, use it, and close it again.
    pub fn open(&self) -> Result<StorageAccessor<'_>, OpenStorageError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        let mut result = self
            .object
            .dispatch(CMD_STORAGE_OPEN)
            .out_objects(1)
            .send(&mut buf)
            .map_err(OpenStorageError::Dispatch)?;

        let raw_object_id = result
            .take_object(0)
            .ok_or(OpenStorageError::MissingObject)?
            .into_raw_object_id();

        Ok(StorageAccessor::new(reanchor_object(
            self.object.domain(),
            raw_object_id,
        )))
    }

    /// Writes `data` at `offset`, opening and closing an accessor around it.
    pub fn write(&self, offset: i64, data: &[u8]) -> Result<(), WriteStorageError> {
        self.open()
            .map_err(WriteStorageError::Open)?
            .write(offset, data)
    }

    /// Reads into `data` from `offset`, opening and closing an accessor around it.
    pub fn read(&self, offset: i64, data: &mut [u8]) -> Result<(), ReadStorageError> {
        self.open()
            .map_err(ReadStorageError::Open)?
            .read(offset, data)
    }

    /// Returns the storage size in bytes, opening and closing an accessor around it.
    pub fn size(&self) -> Result<i64, GetSizeError> {
        self.open().map_err(GetSizeError::Open)?.size()
    }
}

/// Error returned by [`Storage::open`].
#[derive(Debug, thiserror::Error)]
pub enum OpenStorageError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not carry the accessor object.
    #[error("missing accessor object in response")]
    MissingObject,
}

impl ToResultCode for OpenStorageError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingObject => GENERIC_ERROR,
        }
    }
}

/// An `IStorageAccessor`, the object that moves bytes in and out of a
/// [`Storage`].
///
/// Closes the server-side object on drop.
#[derive(Debug)]
pub struct StorageAccessor<'d> {
    object: DomainObject<'d>,
}

impl<'d> StorageAccessor<'d> {
    /// Wraps the domain object this accessor is addressed through.
    pub(super) fn new(object: DomainObject<'d>) -> Self {
        Self { object }
    }

    /// Returns the size of the underlying storage in bytes (cmd 0).
    pub fn size(&self) -> Result<i64, GetSizeError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        let result = self
            .object
            .dispatch(CMD_STORAGE_ACCESSOR_GET_SIZE)
            .out_size(size_of::<i64>())
            .send(&mut buf)
            .map_err(GetSizeError::Dispatch)?;

        if result.data.len() < size_of::<i64>() {
            return Err(GetSizeError::InvalidResponse);
        }

        Ok(*result.value::<i64>())
    }

    /// Writes `data` into the storage at `offset` (cmd 10).
    pub fn write(&self, offset: i64, data: &[u8]) -> Result<(), WriteStorageError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        self.object
            .dispatch(CMD_STORAGE_ACCESSOR_WRITE)
            .in_raw(offset.as_bytes())
            .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
            .send(&mut buf)
            .map_err(WriteStorageError::Dispatch)?;

        Ok(())
    }

    /// Reads from the storage at `offset` into `data` (cmd 11).
    pub fn read(&self, offset: i64, data: &mut [u8]) -> Result<(), ReadStorageError> {
        let mut buf = nx_sys_thread_tls::ipc_buffer();

        self.object
            .dispatch(CMD_STORAGE_ACCESSOR_READ)
            .in_raw(offset.as_bytes())
            .out_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
            .send(&mut buf)
            .map_err(ReadStorageError::Dispatch)?;

        Ok(())
    }
}

/// Error returned by [`StorageAccessor::size`] and [`Storage::size`].
#[derive(Debug, thiserror::Error)]
pub enum GetSizeError {
    /// Failed to open an accessor onto the storage.
    #[error("failed to open storage accessor")]
    Open(#[source] OpenStorageError),
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

impl ToResultCode for GetSizeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::Dispatch(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::InvalidResponse => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`StorageAccessor::write`] and [`Storage::write`].
#[derive(Debug, thiserror::Error)]
pub enum WriteStorageError {
    /// Failed to open an accessor onto the storage.
    #[error("failed to open storage accessor")]
    Open(#[source] OpenStorageError),
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

impl ToResultCode for WriteStorageError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::Dispatch(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`StorageAccessor::read`] and [`Storage::read`].
#[derive(Debug, thiserror::Error)]
pub enum ReadStorageError {
    /// Failed to open an accessor onto the storage.
    #[error("failed to open storage accessor")]
    Open(#[source] OpenStorageError),
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

impl ToResultCode for ReadStorageError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::Dispatch(err) => err.to_rc(),
        }
    }
}
