//! Handle types.

use crate::raw::Handle;

/// A trait for types that can be waited on by the kernel.
pub trait Waitable: _priv::Sealed {
    /// Returns the raw handle of the waitable object.
    fn raw_handle(&self) -> Handle;
}

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
            /// Converts a raw handle to a [`$name`].
            ///
            /// # Safety
            ///
            /// Caller must guarantee that the raw handle is valid.
            pub unsafe fn from_raw(raw: $crate::raw::Handle) -> Self {
                Self(raw)
            }

            /// Returns `true` if the handle is valid.
            pub fn is_valid(&self) -> bool {
                self.0 != $crate::raw::INVALID_HANDLE
            }

            /// Converts the [`$name`] to a raw handle.
            pub fn to_raw(&self) -> $crate::raw::Handle {
                self.0
            }
        }

        impl ::core::cmp::PartialEq<$crate::raw::Handle> for $name {
            fn eq(&self, other: &$crate::raw::Handle) -> bool {
                &self.0 == other
            }
        }

        impl ::core::cmp::PartialEq<$name> for $crate::raw::Handle {
            fn eq(&self, other: &$name) -> bool {
                self == &other.0
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

#[allow(dead_code)]
pub(crate) mod _priv {
    /// A trait that is sealed to prevent external implementations.
    pub trait Sealed {}
}
