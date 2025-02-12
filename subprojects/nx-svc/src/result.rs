//! Result for Horizon OS kernel SVC functions.
//!
//! These codes are used to indicate the success or failure of various operations.
//! They are returned by syscalls and can be used to determine the cause of an error and
//! the module that caused the error.
//!
//! # Structure
//!
//! Result codes have been designed to fit within an _AArch64 MOV_ instruction immediate most of the time
//! (without requiring an additional _MOVK_ instruction).
//!
//! The 32-bit result code is structured as follows:
//!
//! - **Bits 0-8:** Module ID
//! - **Bits 9-21:** Description
//! - **Bits 22-31:** Reserved
//!
//! The bits 22 and above in the error code are reserved and currently unused.
//!
//! # References
//! - [Switchbrew Wiki: SVC](https://switchbrew.org/wiki/SVC)
//! - [Switchbrew Wiki: Error Codes](https://switchbrew.org/wiki/Error_codes)

use crate::error::Module;

/// Type alias for Result with [`Error`] as the error type.
///
/// This is the recommended return type for functions that can fail with a Horizon OS error code.
/// It allows for easy integration with Rust's `?` operator and error handling patterns.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// The raw representation of a result code, containing both success and error states.
///
/// This type is used to represent the raw result code, which can be either a success or an error.
///
/// For error handling with the standard library traits, see [`Result`] and [`Error`].
pub type ResultCode = u32;

/// The error type for Horizon OS result codes.
///
/// This type is used to integrate Horizon OS error codes with Rust's error handling.
/// It provides formatted error messages that include both the error module and description.
///
/// The result code is stored as a raw `u32` value, and it is guaranteed to be non-zero.
///
/// # Formatting
///
/// The error code is formatted as `2XXX-YYYY` where:
///  - `XXX` is `2000` + module number
///  - `YYYY` is the `description`
///
/// ```rust
/// use nx_svc::rc::{Error, ErrorModule};
///
/// let err = Error::from_parts(ErrorModule::Kernel, 404);
///
/// println!("{}", err); // "2001-0404"
/// # assert_eq!(format!("{}", err), "2001-0404");
/// ```
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct Error(raw::ResultCode);

impl Error {
    /// Returns the module that caused the error
    #[inline]
    pub const fn module(&self) -> Module {
        self.0.module()
    }

    /// Returns the description value
    #[inline]
    pub const fn description(&self) -> u32 {
        self.0.description()
    }

    /// Returns the raw value (`u32`) of this error code
    #[inline]
    pub const fn to_raw(self) -> ResultCode {
        self.0.to_raw()
    }
}

impl core::fmt::Display for Error {
    /// Formats the error code as a `2XXX-YYYY` string.
    ///
    /// ```rust
    /// use nx_svc::rc::{Error, ErrorModule};
    ///
    /// let err = Error::from_parts(ErrorModule::Kernel, 500);
    ///
    /// assert_eq!(format!("{}", err), "2001-0500");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:04}-{:04}",
            2000 + self.0.module() as u32,
            self.0.description()
        )
    }
}

impl core::fmt::Debug for Error {
    /// Formats the error code as a debug string.
    ///
    /// ```rust
    /// use nx_svc::rc::{Error, ErrorModule};
    ///
    /// let error = Error::from_parts(ErrorModule::FS, 500);
    ///
    /// assert_eq!(format!("{:?}", error), "Error { code: 2002-0500, module: FS, description: 500, raw: 0x20020500 }");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Error")
            .field("code", &format_args!("{}", self))
            .field("module", &self.0.module())
            .field("description", &self.0.description())
            .field("raw", &format_args!("{:#x}", self.0.to_raw()))
            .finish()
    }
}

impl core::error::Error for Error {}

impl From<raw::ResultCode> for Error {
    /// Converts a [`raw::ResultCode`] into an [`Error`].
    fn from(value: raw::ResultCode) -> Self {
        Self(value)
    }
}

/// Raw representation of the result code
// NOTE: For internal use only
pub(crate) mod raw {
    use crate::error::{IntoDescription, Module};

    /// Successful result code
    const SUCCESS: u32 = 0;

    /// Mask for the module field (9 bits)
    const MODULE_MASK: u32 = 0x1FF;
    /// Mask for the description field (13 bits)
    const DESCRIPTION_MASK: u32 = 0x1FFF;
    /// Shift amount for the description field
    const DESCRIPTION_SHIFT: u32 = 9;

    /// Encapsulates a Horizon OS result code, allowing it to be separated into its constituent fields.
    ///
    /// This is the raw representation of a result code, containing both success and error states.
    ///
    /// For error handling with the standard library traits, see [`Error`].
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(transparent)]
    pub struct ResultCode(u32);

    impl ResultCode {
        /// Creates a new [`ResultCode`] from a raw value
        #[inline]
        pub const fn from_raw(value: u32) -> Self {
            Self(value)
        }

        /// Get the raw value of the [`ResultCode`]
        #[inline]
        pub const fn to_raw(self) -> u32 {
            self.0
        }

        /// Creates a new [`ResultCode`] from a module and description
        #[inline]
        pub fn from_parts(module: Module, description: impl IntoDescription) -> Self {
            let description = description.into_value();

            let module_val = (module as u32) & MODULE_MASK;
            let desc_val = (description & DESCRIPTION_MASK) << DESCRIPTION_SHIFT;
            Self(module_val | desc_val)
        }

        /// Returns true if the [`ResultCode`] represents a success
        #[inline]
        pub const fn is_success(&self) -> bool {
            self.0 == SUCCESS
        }

        /// Returns the module that caused the error
        #[inline]
        pub const fn module(&self) -> Module {
            unsafe { core::mem::transmute::<u32, Module>(self.0 & MODULE_MASK) }
        }

        /// Returns the description value
        #[inline]
        pub const fn description(&self) -> u32 {
            (self.0 >> DESCRIPTION_SHIFT) & DESCRIPTION_MASK
        }
    }

    /// Result for Horizon OS kernel SVC functions.
    pub enum Result {
        /// The operation was successful
        Success,
        /// The operation failed with an error code
        Error(ResultCode),
    }

    impl Result {
        /// Creates a new [`Result`] from a raw result code `u32` value
        pub fn from_raw(raw: u32) -> Self {
            if raw == SUCCESS {
                Result::Success
            } else {
                Result::Error(ResultCode(raw))
            }
        }

        /// Converts this [`Result`] into a [`core::result::Result`] with custom success and error values
        ///
        /// On success, returns `Ok(ok)`. On error, passes the [`ResultCode`] to the provided error mapping
        /// function to produce the error value.
        #[inline]
        pub fn map<T, E>(
            self,
            ok: T,
            err: impl FnOnce(ResultCode) -> E,
        ) -> core::result::Result<T, E> {
            match self {
                Result::Success => Ok(ok),
                Result::Error(rc) => Err(err(rc)),
            }
        }

        /// Converts this [`Result`] into a [`core::result::Result`] with unit success type
        ///
        /// On success, returns `Ok(())`. On error, passes the [`ResultCode`] to the provided error
        /// mapping function to produce the error value.
        #[inline]
        pub fn map_err<E>(self, err: impl FnOnce(ResultCode) -> E) -> core::result::Result<(), E> {
            self.map((), err)
        }

        /// Converts this [`Result`] into a [`core::result::Result`] with custom success and error values
        ///
        /// Similar to [`map`], but passes only the description value from the [`ResultCode`] to the error
        /// mapping function, discarding the module information.
        #[inline]
        pub fn map_desc<T, E>(
            self,
            ok: T,
            err: impl FnOnce(u32) -> E,
        ) -> core::result::Result<T, E> {
            self.map(ok, |rc| err(rc.description()))
        }
    }
}
