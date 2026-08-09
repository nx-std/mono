//! Wire structures the auth applet reads and writes.
//!
//! Both storages are fixed-layout payloads exchanged verbatim through an
//! `IStorage`, so they are modelled as `repr(C)` structs and converted with
//! zerocopy rather than serialised field by field. The header every library
//! applet reads first is [`LibraryAppletArgs`](nx_service_applet::LibraryAppletArgs),
//! which the launch sequence pushes on this crate's behalf.

use static_assertions::const_assert_eq;

/// Which Parental Controls screen the applet opens on.
///
/// libnx calls this `PctlAuthType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PctlAuthType {
    /// Ask the user for the PIN, `ShowParentalAuthentication`.
    Show = 0,
    /// Register a PIN, `RegisterParentalPasscode`.
    RegisterPasscode = 1,
    /// Change the PIN, `ChangeParentalPasscode`.
    ChangePasscode = 2,
}

impl PctlAuthType {
    /// Returns the raw word this type is written as.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Argument storage for the auth applet.
///
/// libnx calls this `PctlAuthArg`.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct PctlAuthArg {
    _unk_x0: u32,
    /// Which screen to open, see [`PctlAuthType`].
    pub ty: u32,
    /// First argument byte. On the [`Show`](PctlAuthType::Show) screen, zero
    /// temporarily disables Parental Controls and one validates the PIN the user
    /// enters.
    pub arg0: u8,
    /// Second argument byte. The applet reads it from `[4.0.0+]`, but no meaning
    /// has been established for it, so it keeps its libnx name.
    pub arg1: u8,
    /// Third argument byte. The applet reads it from `[4.0.0+]`, but no meaning
    /// has been established for it, so it keeps its libnx name.
    pub arg2: u8,
    _pad: u8,
}

const_assert_eq!(core::mem::size_of::<PctlAuthArg>(), 0xC);

impl PctlAuthArg {
    /// Builds an argument storage opening on `ty`, with every argument byte
    /// cleared.
    ///
    /// The caller fills the bytes its screen accepts; see
    /// [`ParentalAuth`](crate::ParentalAuth), which does so from the shape of
    /// the request. libnx does the same, zeroing the struct at each entry point
    /// and setting only what that entry point was given.
    pub const fn new(ty: PctlAuthType) -> Self {
        Self {
            _unk_x0: 0,
            ty: ty.as_raw(),
            arg0: 0,
            arg1: 0,
            arg2: 0,
            _pad: 0,
        }
    }
}

/// Reply storage the auth applet pops back.
///
/// libnx names neither the type nor the field, reading the reply into a bare
/// `Result` and returning it as its own. A zero means the user satisfied the
/// screen; any other value is the applet's own failure code.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct PctlAuthReply {
    /// The result code the applet reported.
    pub result: zerocopy::little_endian::U32,
}

const_assert_eq!(core::mem::size_of::<PctlAuthReply>(), 4);
