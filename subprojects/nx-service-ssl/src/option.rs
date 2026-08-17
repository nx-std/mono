//! Reading a word as one of the options the service defines.
//!
//! Every option in this crate is a small integer the service assigns meaning to, and a C caller
//! names one by passing that integer. Each of the enums in [`certificate`](crate::certificate),
//! [`connection`](crate::connection), [`context`](crate::context) and [`service`](crate::service)
//! is therefore reached through a `TryFrom<u32>` rather than taken by value across `extern "C"`,
//! where a word naming no variant would be undefined behaviour.
//!
//! All of those conversions fail the same way and with the same amount to say, so they share the
//! one error here. It sits in a module of its own rather than in any of the four, because a module
//! that declares the others must not also hold what they need: `lib.rs` declares all four, so an
//! error living there would have every one of them reaching back up into their own parent.

/// Error returned when a word names no variant of the option it was read as.
///
/// One type serves every option in the crate, because they all fail for the same reason: a caller
/// passed a number, and the service defines what the numbers mean. Which option was being read is
/// the conversion's own type, so the error does not repeat it.
#[derive(Debug, thiserror::Error)]
#[error("no option is numbered {value:#x}")]
pub struct UnknownOption {
    /// The value that was offered.
    pub value: u32,
}
