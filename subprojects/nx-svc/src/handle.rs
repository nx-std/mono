//! Handle types.

use crate::raw::Handle;

/// Errors returned when wrapping a raw handle in one of this crate's handle types.
///
/// Occurs when the raw value is [`INVALID_HANDLE`], which names no kernel
/// object. Nothing is consumed by a rejected conversion, so the caller still
/// owns whatever the raw value came from.
///
/// [`INVALID_HANDLE`]: crate::raw::INVALID_HANDLE
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("The raw handle names no kernel object")]
pub struct InvalidHandleError;

/// A trait for types that can be waited on by the kernel.
pub trait Waitable: _priv::Sealed {
    /// Returns the raw handle of the waitable object.
    fn raw_handle(&self) -> Handle;
}

/// Marker trait for synchronization objects that support `svcResetSignal`.
///
/// Only these kernel object types can be reset:
/// - `KReadableEvent` (user events)
/// - `KInterruptEvent` (interrupt events)
/// - `KProcess` (process handles)
///
/// This trait is sealed to prevent external implementations.
pub trait Reset: Waitable + _priv::Sealed {}

/// Internal macro to generate [`Handle`] newtypes with common helpers.
///
/// [`Handle`]: crate::raw::Handle
macro_rules! define_handle_type {
    {
        $(#[$meta:meta])* $vis:vis struct $name:ident
    } => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $vis struct $name($crate::raw::Handle);

        impl $name {
            /// Creates a new handle from a raw value, returning `None` if invalid.
            ///
            /// Delegates to the [`TryFrom`] impl, which is where the invariant is
            /// checked; this form exists for call sites that have no use for the
            /// rejection reason.
            pub fn new(raw: $crate::raw::Handle) -> Option<Self> {
                <Self as ::core::convert::TryFrom<$crate::raw::Handle>>::try_from(raw).ok()
            }

            /// Wraps a raw handle without checking that it names a live kernel object.
            ///
            /// The caller must ensure `raw` is a handle the kernel issued to this process and
            /// has not closed. Nothing here can check that, since only the kernel knows which
            /// handle numbers are live. A stale or fabricated handle is answered with
            /// `InvalidHandle` by the SVC it reaches rather than faulting, which is why this
            /// is a safe function; [`TryFrom`] is the checked constructor, and it rejects only
            /// the reserved invalid value.
            pub const fn from_raw_unchecked(raw: $crate::raw::Handle) -> Self {
                Self(raw)
            }

            /// Returns `true` if the handle is valid.
            pub const fn is_valid(&self) -> bool {
                self.0 != $crate::raw::INVALID_HANDLE
            }

            /// Converts the [`$name`] to a raw handle.
            pub const fn to_raw(&self) -> $crate::raw::Handle {
                self.0
            }
        }

        impl ::core::convert::TryFrom<$crate::raw::Handle> for $name {
            type Error = $crate::handle::InvalidHandleError;

            fn try_from(
                raw: $crate::raw::Handle,
            ) -> ::core::result::Result<Self, Self::Error> {
                if raw == $crate::raw::INVALID_HANDLE {
                    Err($crate::handle::InvalidHandleError)
                } else {
                    Ok(Self(raw))
                }
            }
        }

        impl ::core::cmp::PartialEq<$crate::raw::Handle> for $name {
            #[inline]
            fn eq(&self, other: &$crate::raw::Handle) -> bool {
                &self.0 == other
            }
        }

        impl ::core::cmp::PartialEq<$name> for $crate::raw::Handle {
            #[inline]
            fn eq(&self, other: &$name) -> bool {
                self == &other.0
            }
        }

        impl ::core::convert::From<$name> for $crate::raw::Handle {
            #[inline]
            fn from(handle: $name) -> Self {
                handle.0
            }
        }
    };
}

/// Helper macro that creates a new handle *type* that is also [`Waitable`].
///
/// The macro expands to a new-type wrapper around [`Handle`] (complete with the helpers from
/// [`define_handle_type!`]) and automatically adds a [`Waitable`] implementation.
///
/// [`Handle`]: crate::raw::Handle
macro_rules! define_waitable_handle_type {
    {
        $(#[$meta:meta])* $vis:vis struct $name:ident
    } => {
        define_handle_type! {
            $(#[$meta])* $vis struct $name
        }

        impl $crate::handle::Waitable for $name {
            #[inline]
            fn raw_handle(&self) -> $crate::raw::Handle {
                self.0
            }
        }

        impl $crate::handle::_priv::Sealed for $name {}
    };
}

/// Helper macro that creates a new handle *type* that is both [`Waitable`] and [`Reset`].
///
/// The macro expands to a new-type wrapper around [`Handle`] (via [`define_waitable_handle_type!`])
/// and automatically adds both [`Waitable`] and [`Reset`] implementations.
///
/// Only use this macro for handle types that represent kernel objects supporting `svcResetSignal`:
/// - `KReadableEvent` (user events)
/// - `KInterruptEvent` (interrupt events)
/// - `KProcess` (process handles)
///
/// [`Handle`]: crate::raw::Handle
/// [`Reset`]: crate::handle::Reset
macro_rules! define_reset_handle_type {
    {
        $(#[$meta:meta])* $vis:vis struct $name:ident
    } => {
        define_waitable_handle_type! {
            $(#[$meta])* $vis struct $name
        }

        impl $crate::handle::Reset for $name {}
    };
}

#[allow(dead_code)]
pub(crate) mod _priv {
    /// A trait that is sealed to prevent external implementations.
    pub trait Sealed {}
}
