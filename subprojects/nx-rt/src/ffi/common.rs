//! Common utilities for FFI modules

use core::cell::UnsafeCell;

use nx_sf::cmif;

/// Generic error code for FFI when no specific result code is available.
pub const GENERIC_ERROR: u32 = 0xFFFF;

/// libnx error enumeration for MAKERESULT(Module_Libnx, error).
#[repr(u32)]
pub enum LibnxError {
    NotInitialized = 2,
    IncompatSysVer = 100,
}

/// Constructs a libnx result code.
pub const fn libnx_error(err: LibnxError) -> u32 {
    const MODULE_LIBNX: u32 = 345;
    (MODULE_LIBNX & 0x1FF) | ((err as u32 & 0x1FFF) << 9)
}

/// Wrapper to make UnsafeCell Sync for static storage.
#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: Access is synchronized by SM_SESSION lock in service_manager.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Converts a `DispatchError` to a raw result code.
pub fn dispatch_error_to_rc(err: nx_sf::service::DispatchError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_sf::service::DispatchError::SendRequest(e) => e.to_rc(),
        nx_sf::service::DispatchError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

/// Converts a `ConvertToDomainError` to a raw result code.
pub fn convert_to_domain_error_to_rc(err: nx_sf::service::ConvertToDomainError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_sf::service::ConvertToDomainError::SendRequest(e) => e.to_rc(),
        nx_sf::service::ConvertToDomainError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}
