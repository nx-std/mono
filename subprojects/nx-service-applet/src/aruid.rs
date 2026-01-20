//! Applet Resource User ID (ARUID) type.

/// Sentinel value indicating no ARUID is provided.
///
/// When passed to services, ARUID 0 skips client validation - the service
/// will use the process ID from the kernel instead of validating against
/// a client-provided ARUID.
pub const NO_ARUID: u64 = 0;

/// Applet Resource User ID - identifies an applet to system services.
///
/// This ID is assigned by the system during applet initialization and is used
/// by various services (HID, audio, NV, etc.) to identify the applet making
/// requests.
///
/// Use `Option<Aruid>` for contexts where an ARUID may not be available.
/// When `None`, services receive [`NO_ARUID`] (0) which skips validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Aruid(u64);

impl Aruid {
    /// Creates a new ARUID from a raw value.
    ///
    /// Returns `None` if the value is 0 ([`NO_ARUID`]).
    #[inline]
    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Returns the raw u64 value for FFI/IPC calls.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

impl From<Aruid> for u64 {
    #[inline]
    fn from(aruid: Aruid) -> Self {
        aruid.0
    }
}
