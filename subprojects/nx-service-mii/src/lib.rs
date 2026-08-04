//! Mii service (`mii:*`) implementation.
//!
//! Provides access to the Mii database for querying, listing, and generating
//! random Mii character data on the Nintendo Switch.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif_system`] / [`connect_cmif_user`]
//! obtain the root session, then [`MiiService::open_database`] returns a
//! [`MiiDatabase`] with its own independent session handle.
//!
//! ## Divergence from libnx
//!
//! libnx's `mii.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD` and selects the service name via
//! `MiiServiceType`. This crate follows the convention of the other
//! `nx-service-*` crates: connect once via [`connect_cmif_system`] or
//! [`connect_cmif_user`], then call methods directly.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        DispatchError,
        OwnedSessionHandle,
        Session,
    },
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::OpenDatabaseError,
    proto::{
        SERVICE_NAME_SYSTEM,
        SERVICE_NAME_USER,
    },
    types::{
        MiiAge,
        MiiCharInfo,
        MiiCreateId,
        MiiFaceColor,
        MiiGender,
        MiiNfpStoreDataExtension,
        MiiSourceFlag,
        MiiSpecialKeyCode,
        MiiStoreData,
        MiiVer3StoreData,
    },
};

/// Mii root service wrapper.
///
/// Use [`open_database`](Self::open_database) to create a database sub-object.
#[repr(transparent)]
pub struct MiiService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for MiiService {}
unsafe impl Sync for MiiService {}

impl MiiService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Opens a Mii database.
    ///
    /// `key_code` selects normal or special Miis.
    pub fn open_database(
        &self,
        key_code: MiiSpecialKeyCode,
    ) -> Result<MiiDatabase, OpenDatabaseError> {
        let raw_handle = cmif::open_database(&self.0, key_code as u32)?;

        // SAFETY: The server returned a freshly opened database session in this reply, so the
        // `Session` below is its sole owner.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        let service = Session::new(handle, 0);

        Ok(MiiDatabase(service))
    }
}

/// Mii database session wrapper.
///
/// Obtained via [`MiiService::open_database`]. Owns its own independent
/// session handle.
#[repr(transparent)]
pub struct MiiDatabase(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for MiiDatabase {}
unsafe impl Sync for MiiDatabase {}

impl MiiDatabase {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Checks if the database has been updated for the given source flag.
    #[inline]
    pub fn is_updated(&self, flag: MiiSourceFlag) -> Result<bool, DispatchError> {
        cmif::db_is_updated(&self.0, flag)
    }

    /// Checks if the database is full.
    #[inline]
    pub fn is_full(&self) -> Result<bool, DispatchError> {
        cmif::db_is_full(&self.0)
    }

    /// Gets the number of Miis matching a source flag.
    #[inline]
    pub fn get_count(&self, flag: MiiSourceFlag) -> Result<i32, DispatchError> {
        cmif::db_get_count(&self.0, flag)
    }

    /// Reads Mii character info entries matching a source flag.
    ///
    /// Fills `buffer` with up to `buffer.len()` entries. Returns the number
    /// of entries actually written.
    #[inline]
    pub fn get1(
        &self,
        flag: MiiSourceFlag,
        buffer: &mut [MiiCharInfo],
    ) -> Result<i32, DispatchError> {
        cmif::db_get1(&self.0, flag, buffer)
    }

    /// Generates a random Mii character info.
    ///
    /// The generated Mii is not registered in the console database.
    #[inline]
    pub fn build_random(
        &self,
        age: MiiAge,
        gender: MiiGender,
        face_color: MiiFaceColor,
    ) -> Result<MiiCharInfo, DispatchError> {
        cmif::db_build_random(&self.0, age as u32, gender as u32, face_color as u32)
    }
}

/// Connects to the Mii system service (`mii:e`) using CMIF.
pub fn connect_cmif_system(sm: &SmService) -> Result<MiiService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_SYSTEM)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(MiiService(service))
}

/// Connects to the Mii user service (`mii:u`) using CMIF.
pub fn connect_cmif_user(sm: &SmService) -> Result<MiiService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_USER)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(MiiService(service))
}

/// Error returned by [`connect_cmif_system`] and [`connect_cmif_user`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get mii service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
