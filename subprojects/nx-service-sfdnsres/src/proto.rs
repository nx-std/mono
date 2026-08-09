//! Protocol constants and wire-format types for the sfdnsres service.

use core::mem::size_of;

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;
use zerocopy::{
    FromBytes,
    Immutable,
    IntoBytes,
    KnownLayout,
};

/// Service name for the DNS resolver service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("sfdnsres");

/// Command ID for GetHostByNameRequest.
pub const CMD_GET_HOST_BY_NAME: u32 = 2;

/// Command ID for GetHostByAddrRequest.
pub const CMD_GET_HOST_BY_ADDR: u32 = 3;

/// Command ID for GetHostStringErrorRequest.
pub const CMD_GET_HOST_STRING_ERROR: u32 = 4;

/// Command ID for GetGaiStringErrorRequest.
pub const CMD_GET_GAI_STRING_ERROR: u32 = 5;

/// Command ID for GetAddrInfoRequest.
pub const CMD_GET_ADDR_INFO: u32 = 6;

/// Command ID for GetNameInfoRequest.
pub const CMD_GET_NAME_INFO: u32 = 7;

/// Command ID for GetCancelHandleRequest.
pub const CMD_GET_CANCEL_HANDLE: u32 = 8;

/// Command ID for CancelRequest.
pub const CMD_CANCEL: u32 = 9;

/// Opaque cancel-token issued by `GetCancelHandleRequest` and consumed by
/// `CancelRequest` / pending resolver calls.
///
/// Wrapped in a newtype so it cannot be confused with arbitrary `u32` values
/// (e.g. `errno`, flags, command IDs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CancelHandle(u32);

impl CancelHandle {
    /// Wraps a raw cancel-token value as returned by the service.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw cancel-token value for wire serialization.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

bitflags::bitflags! {
    /// Lookup flags for `GetNameInfoRequest` (cmd 7).
    ///
    /// The named flags mirror the BSD `NI_*` set; combine them with `|`. A
    /// bitset rather than a bare `u32` so it cannot be confused with the other
    /// `u32` words this protocol carries — a cancel token, a serialized size.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct NameInfoFlags: u32 {
        /// `NI_NOFQDN` — return only the hostname portion of a fully-qualified
        /// name.
        const NOFQDN = 0x0000_0001;
        /// `NI_NUMERICHOST` — return the address in numeric form, skip the
        /// name lookup.
        const NUMERICHOST = 0x0000_0002;
        /// `NI_NAMEREQD` — fail if the host name cannot be resolved.
        const NAMEREQD = 0x0000_0004;
        /// `NI_NUMERICSERV` — return the service in numeric (port) form.
        const NUMERICSERV = 0x0000_0008;
        /// `NI_DGRAM` — the service is datagram-based (UDP rather than TCP).
        const DGRAM = 0x0000_0010;
        /// `NI_NUMERICSCOPE` — return the scope identifier in numeric form.
        const NUMERICSCOPE = 0x0000_0020;
    }
}

impl NameInfoFlags {
    /// Adopts a raw `NI_*` bitmask as a C caller supplied it.
    ///
    /// Bits with no name here are kept rather than dropped: they are the
    /// caller's request, and the resolver is what decides whether it
    /// understands them.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }

    /// The raw `NI_*` bitmask, for wire serialization.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.bits()
    }
}

/// Input payload for `GetHostByNameRequest` (cmd 2).
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetHostByNameIn {
    /// Whether to use the NSD (Network Service Discovery) backend (`u8` bool widened to `u32`).
    pub use_nsd: u32,
    /// Cancel-token, or `0` for no cancellation.
    pub cancel_handle: u32,
    /// PID placeholder — actual PID is delivered via the HIPC send-PID flag.
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<GetHostByNameIn>(), 16);

/// Output payload for `GetHostByNameRequest` (cmd 2).
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetHostByNameOut {
    /// `h_errno` value from the resolver.
    pub h_errno: u32,
    /// `errno` value from the resolver.
    pub errno: u32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}
const_assert_eq!(size_of::<GetHostByNameOut>(), 12);

/// Input payload for `GetHostByAddrRequest` (cmd 3).
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetHostByAddrIn {
    /// Length of the input address buffer.
    pub addr_len: u32,
    /// Address family / type, passed through to the service unchanged.
    pub addr_type: u32,
    /// Cancel-token, or `0` for no cancellation.
    pub cancel_handle: u32,
    /// Padding to keep `pid_placeholder` 8-byte aligned.
    pub _padding: u32,
    /// PID placeholder (this command does not send a PID, but the field is still
    /// declares the field — kept for layout parity).
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<GetHostByAddrIn>(), 24);

/// Output payload for `GetHostByAddrRequest` (cmd 3).
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetHostByAddrOut {
    /// `h_errno` value from the resolver.
    pub h_errno: u32,
    /// `errno` value from the resolver.
    pub errno: u32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}
const_assert_eq!(size_of::<GetHostByAddrOut>(), 12);

/// Input payload for `GetAddrInfoRequest` (cmd 6).
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetAddrInfoIn {
    /// Whether to use the NSD backend (`u8` bool widened to `u32`).
    pub use_nsd: u32,
    /// Cancel-token, or `0` for no cancellation.
    pub cancel_handle: u32,
    /// PID placeholder — actual PID via send-PID flag.
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<GetAddrInfoIn>(), 16);

/// Output payload for `GetAddrInfoRequest` (cmd 6).
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetAddrInfoOut {
    /// `errno` value from the resolver.
    pub errno: u32,
    /// `getaddrinfo` return code.
    pub ret: i32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}
const_assert_eq!(size_of::<GetAddrInfoOut>(), 12);

/// Input payload for `GetNameInfoRequest` (cmd 7).
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetNameInfoIn {
    /// `getnameinfo` flags.
    pub flags: u32,
    /// Cancel-token, or `0` for no cancellation.
    pub cancel_handle: u32,
    /// PID placeholder — actual PID via send-PID flag.
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<GetNameInfoIn>(), 16);

/// Output payload for `GetNameInfoRequest` (cmd 7).
#[derive(Debug, Clone, Copy, Default, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct GetNameInfoOut {
    /// `errno` value from the resolver.
    pub errno: u32,
    /// `getnameinfo` return code.
    pub ret: i32,
}
const_assert_eq!(size_of::<GetNameInfoOut>(), 8);

/// Input payload for `CancelRequest` (cmd 9).
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct CancelIn {
    /// Cancel-token to cancel.
    pub cancel_handle: u32,
    /// Padding for 8-byte alignment of the PID placeholder.
    pub _padding: u32,
    /// PID placeholder — actual PID via send-PID flag.
    pub pid_placeholder: u64,
}
const_assert_eq!(size_of::<CancelIn>(), 16);
