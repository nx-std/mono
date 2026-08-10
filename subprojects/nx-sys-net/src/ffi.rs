//! The C socket API.
//!
//! These are the symbols a C program calls when it opens a socket, and the ones the override
//! script redirects onto this crate. Together they replace the C socket driver: the BSD calls
//! themselves, the driver lifecycle around them, and the readiness and control calls that are
//! declared here rather than in the C standard library because nothing else on this platform
//! defines them.
//!
//! ## The shape every export shares
//!
//! Each export is a hard shell over a soft core ([validate at the
//! edge](../../../docs/code/principle-validate-at-edge.md)):
//!
//! 1. Turn the C arguments into this crate's types, refusing anything that does not convert.
//! 2. Map the caller's descriptor to the service's, through [`crate::device::sock_of`].
//! 3. Run the command.
//! 4. Report the outcome the way C expects: the value on success, and on failure `-1` with the
//!    reason left in the calling thread's `errno`.
//!
//! Step 4 happens in exactly one place per direction, in [`errno`], so no export decides for
//! itself what a failure looks like.
//!
//! ## What is deliberately absent
//!
//! `read`, `write` and `close` are not here. They are descriptor operations, so the C standard
//! library resolves them through the descriptor table and they arrive at
//! [`crate::device::SocketFile`] without passing through this module. Defining them here would put
//! a second implementation of each beside the one the table already dispatches to.

pub mod abi;
pub mod control;
pub mod descriptor;
pub mod driver;
pub mod endpoint;
pub mod errno;
pub mod readiness;
pub mod transfer;
