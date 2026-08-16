//! What the running firmware offers.
//!
//! The `ssl` service grew over nine releases, and upstream answers that by comparing a version at
//! each of the twenty-eight commands that care. This module asks the question once per *capability*
//! instead, so an entry point reads a named fact and no version literal appears beside a command.
//!
//! Two things follow from that. A release that adds a command needs one predicate here rather than
//! a comparison threaded through the call, and the commands that arrived together say so by
//! sharing a predicate: eleven of them arrived in `[16.0.0]` with DTLS, and they ask
//! [`offers_dtls`].
//!
//! The version is a run-constant. The entry crate stores it once during environment setup and it
//! cannot change while the process lives, so nothing here recomputes something that could have
//! moved.

use nx_rt_core::env::hos_version::{
    self,
    HosVersion,
};

/// Whether the service reports how many certificates it wrote, and accepts a CRL.
///
/// `[3.0.0]` reshaped `GetCertificates` to return a count and added `ImportCrl` and `RemoveCrl`.
pub(crate) fn offers_certificate_count() -> bool {
    at_least(3, 0, 0)
}

/// The interface revision to declare during initialization, if the service accepts one.
///
/// `SetInterfaceVersion` arrived in `[3.0.0]`, and the value the service expects rose twice after
/// that. `None` is the firmware that has no such command, which is the one case where not sending
/// it is correct rather than a downgrade.
pub(crate) fn interface_version() -> Option<u32> {
    if at_least(6, 0, 0) {
        Some(0x3)
    } else if at_least(5, 0, 0) {
        Some(0x2)
    } else if at_least(3, 0, 0) {
        Some(0x1)
    } else {
        None
    }
}

/// Whether the negotiated cipher can be read back (`[4.0.0]`).
pub(crate) fn offers_cipher_info() -> bool {
    at_least(4, 0, 0)
}

/// Whether the service-wide session cache can be flushed (`[5.0.0]`).
pub(crate) fn offers_session_cache_flush() -> bool {
    at_least(5, 0, 0)
}

/// Whether the service carries debug options (`[6.0.0]`).
pub(crate) fn offers_debug_option() -> bool {
    at_least(6, 0, 0)
}

/// Whether ALPN can be negotiated (`[9.0.0]`).
pub(crate) fn offers_alpn() -> bool {
    at_least(9, 0, 0)
}

/// Whether the TLS 1.2 fallback flag can be cleared (`[14.0.0]`).
pub(crate) fn offers_tls12_fallback_flag() -> bool {
    at_least(14, 0, 0)
}

/// Whether the system interface exists (`[15.0.0]`).
///
/// This is what `ssl:s` and the commands only that variant answers are gated on, including the
/// thread core mask and `CreateConnectionForSystem`.
pub(crate) fn offers_system_interface() -> bool {
    at_least(15, 0, 0)
}

/// Whether DTLS and the commands that arrived with it exist (`[16.0.0]`).
pub(crate) fn offers_dtls() -> bool {
    at_least(16, 0, 0)
}

/// Whether a private option carries a value rather than a flag (`[17.0.0]`).
///
/// The command is older than this, but its payload changed shape, so what a caller needs to know
/// is which of the two layouts to send.
pub(crate) fn offers_private_option_value() -> bool {
    at_least(17, 0, 0)
}

/// The one place a firmware version is compared.
fn at_least(major: u8, minor: u8, micro: u8) -> bool {
    hos_version::get() >= HosVersion::new(major, minor, micro)
}
