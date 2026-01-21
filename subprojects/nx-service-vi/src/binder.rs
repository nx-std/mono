//! Binder protocol for IGraphicBufferProducer communication.
//!
//! The Binder protocol is used to communicate with graphics buffer producers
//! via parcel-based transactions.

use nx_sf::service::Service;
use nx_svc::raw::Handle as RawHandle;

use crate::{
    cmif,
    parcel::{PARCEL_MAX_PAYLOAD, Parcel, ParcelHeader},
    types::BinderObjectId,
};

/// Binder session for IGraphicBufferProducer communication.
pub struct Binder {
    /// Binder object ID.
    id: BinderObjectId,
    /// Whether the binder session has been initialized.
    initialized: bool,
}

impl Binder {
    /// Creates a new Binder with the given object ID.
    ///
    /// The binder is not yet initialized; call [`init_session`](Self::init_session)
    /// to initialize it.
    pub fn create(id: BinderObjectId) -> Self {
        Self {
            id,
            initialized: false,
        }
    }

    /// Returns the binder object ID.
    #[inline]
    pub fn id(&self) -> BinderObjectId {
        self.id
    }

    /// Returns whether the binder session is initialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initializes the binder session.
    ///
    /// This increments both weak and strong references on the binder object.
    pub fn init_session(&mut self, relay: &Service) -> Result<(), InitSessionError> {
        if self.initialized {
            return Err(InitSessionError::AlreadyInitialized);
        }

        // Increase weak reference
        cmif::binder::adjust_refcount(relay.session, self.id, 1, 0)
            .map_err(InitSessionError::IncreaseWeakRef)?;

        // Increase strong reference
        if let Err(e) = cmif::binder::adjust_refcount(relay.session, self.id, 1, 1) {
            // Rollback weak ref on failure
            let _ = cmif::binder::adjust_refcount(relay.session, self.id, -1, 0);
            return Err(InitSessionError::IncreaseStrongRef(e));
        }

        self.initialized = true;
        Ok(())
    }

    /// Closes the binder session.
    ///
    /// This decrements both strong and weak references.
    pub fn close(&mut self, relay: &Service) {
        if !self.initialized {
            return;
        }

        // Decrease strong reference
        let _ = cmif::binder::adjust_refcount(relay.session, self.id, -1, 1);
        // Decrease weak reference
        let _ = cmif::binder::adjust_refcount(relay.session, self.id, -1, 0);

        self.initialized = false;
    }

    /// Performs a binder transaction.
    ///
    /// Sends `in_parcel` and receives the response in `out_parcel`.
    pub fn transact(
        &self,
        relay: &Service,
        code: u32,
        in_parcel: &Parcel,
        out_parcel: &mut Parcel,
        flags: u32,
    ) -> Result<(), TransactError> {
        if !self.initialized {
            return Err(TransactError::NotInitialized);
        }

        // Build the input buffer with header
        let mut in_buf = [0u8; PARCEL_MAX_PAYLOAD];
        let payload_size = in_parcel.payload_size();

        if payload_size > PARCEL_MAX_PAYLOAD - ParcelHeader::SIZE {
            return Err(TransactError::InputTooLarge);
        }

        // Write header
        let header = ParcelHeader {
            payload_size: payload_size as u32,
            payload_off: ParcelHeader::SIZE as u32,
            objects_size: 0,
            objects_off: (ParcelHeader::SIZE + payload_size) as u32,
        };

        // SAFETY: Header is repr(C) and properly sized.
        unsafe {
            core::ptr::write_unaligned(in_buf.as_mut_ptr().cast::<ParcelHeader>(), header);
        }

        // Copy payload after header
        in_buf[ParcelHeader::SIZE..ParcelHeader::SIZE + payload_size]
            .copy_from_slice(in_parcel.payload());

        let total_in_size = ParcelHeader::SIZE + payload_size;

        // Output buffer
        let mut out_buf = [0u8; PARCEL_MAX_PAYLOAD];

        // Perform transaction
        cmif::binder::transact_parcel(
            relay.session,
            self.id,
            code,
            &in_buf[..total_in_size],
            &mut out_buf,
            flags,
        )?;

        // Parse output header
        // SAFETY: We're reading from the start of out_buf which has enough space.
        let out_header =
            unsafe { core::ptr::read_unaligned(out_buf.as_ptr().cast::<ParcelHeader>()) };

        // Validate header
        if out_header.payload_size as usize > PARCEL_MAX_PAYLOAD {
            return Err(TransactError::InvalidResponse);
        }
        if out_header.payload_off as usize > PARCEL_MAX_PAYLOAD {
            return Err(TransactError::InvalidResponse);
        }
        if (out_header.payload_off + out_header.payload_size) as usize > PARCEL_MAX_PAYLOAD {
            return Err(TransactError::InvalidResponse);
        }

        // Copy payload to output parcel
        let payload_start = out_header.payload_off as usize;
        let payload_end = payload_start + out_header.payload_size as usize;
        out_parcel
            .payload_mut()
            .copy_from_slice(&out_buf[..PARCEL_MAX_PAYLOAD]);
        out_parcel.set_payload_size(out_header.payload_size as usize);
        out_parcel.reset_read_pos();

        // Adjust read position to skip header offset (parcel data starts at payload_off)
        // Actually, we copy the raw payload, so position should start at 0 relative to payload
        // Let me reconsider: we want out_parcel to contain just the payload portion
        let payload_data = &out_buf[payload_start..payload_end];
        out_parcel.payload_mut()[..payload_data.len()].copy_from_slice(payload_data);
        out_parcel.set_payload_size(payload_data.len());
        out_parcel.reset_read_pos();

        Ok(())
    }

    /// Gets a native handle from the binder.
    ///
    /// Used to get fence sync event handles.
    pub fn get_native_handle(
        &self,
        relay: &Service,
        inval: u32,
    ) -> Result<RawHandle, GetNativeHandleError> {
        if !self.initialized {
            return Err(GetNativeHandleError::NotInitialized);
        }

        cmif::binder::get_native_handle(relay.session, self.id, inval)
            .map_err(GetNativeHandleError::Cmif)
    }
}

/// Error from [`Binder::init_session`].
#[derive(Debug, thiserror::Error)]
pub enum InitSessionError {
    /// Binder session is already initialized.
    #[error("binder session already initialized")]
    AlreadyInitialized,
    /// Failed to increase weak reference.
    #[error("failed to increase weak reference")]
    IncreaseWeakRef(#[source] cmif::binder::AdjustRefcountError),
    /// Failed to increase strong reference.
    #[error("failed to increase strong reference")]
    IncreaseStrongRef(#[source] cmif::binder::AdjustRefcountError),
}

/// Error from [`Binder::transact`].
#[derive(Debug, thiserror::Error)]
pub enum TransactError {
    /// Binder session not initialized.
    #[error("binder session not initialized")]
    NotInitialized,
    /// Input parcel is too large.
    #[error("input parcel too large")]
    InputTooLarge,
    /// Invalid response from binder.
    #[error("invalid response from binder")]
    InvalidResponse,
    /// CMIF transaction failed.
    #[error("CMIF transaction failed")]
    Cmif(#[from] cmif::binder::TransactParcelError),
}

/// Error from [`Binder::get_native_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetNativeHandleError {
    /// Binder session not initialized.
    #[error("binder session not initialized")]
    NotInitialized,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] cmif::binder::GetNativeHandleError),
}

/// Binder error codes (Android-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BinderError {
    /// Permission denied.
    #[error("permission denied")]
    PermissionDenied,
    /// Name not found.
    #[error("name not found")]
    NameNotFound,
    /// Would block.
    #[error("would block")]
    WouldBlock,
    /// No memory.
    #[error("no memory")]
    NoMemory,
    /// Already exists.
    #[error("already exists")]
    AlreadyExists,
    /// Not initialized.
    #[error("not initialized")]
    NoInit,
    /// Bad value.
    #[error("bad value")]
    BadValue,
    /// Dead object.
    #[error("dead object")]
    DeadObject,
    /// Invalid operation.
    #[error("invalid operation")]
    InvalidOperation,
    /// Not enough data.
    #[error("not enough data")]
    NotEnoughData,
    /// Unknown transaction.
    #[error("unknown transaction")]
    UnknownTransaction,
    /// Bad index.
    #[error("bad index")]
    BadIndex,
    /// Timed out.
    #[error("timed out")]
    TimedOut,
    /// FDs not allowed.
    #[error("FDs not allowed")]
    FdsNotAllowed,
    /// Failed transaction.
    #[error("failed transaction")]
    FailedTransaction,
    /// Bad type.
    #[error("bad type")]
    BadType,
    /// Unknown error code.
    #[error("unknown binder error: {0}")]
    Unknown(i32),
}

impl BinderError {
    /// Converts a raw binder error code to a Result.
    ///
    /// Returns `Ok(())` if the code is non-negative (success).
    pub fn from_code(code: i32) -> Result<(), Self> {
        if code >= 0 {
            return Ok(());
        }

        Err(match -code {
            1 => Self::PermissionDenied,
            2 => Self::NameNotFound,
            11 => Self::WouldBlock,
            12 => Self::NoMemory,
            17 => Self::AlreadyExists,
            19 => Self::NoInit,
            22 => Self::BadValue,
            32 => Self::DeadObject,
            38 => Self::InvalidOperation,
            61 => Self::NotEnoughData,
            74 => Self::UnknownTransaction,
            75 => Self::BadIndex,
            110 => Self::TimedOut,
            // Special Android binder error codes (INT32_MIN based)
            x if x == (i32::MIN + 7).wrapping_neg() => Self::FdsNotAllowed,
            x if x == (i32::MIN + 2).wrapping_neg() => Self::FailedTransaction,
            x if x == (i32::MIN + 1).wrapping_neg() => Self::BadType,
            _ => Self::Unknown(code),
        })
    }
}
