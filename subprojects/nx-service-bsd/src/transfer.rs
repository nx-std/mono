//! How data moves on a socket: what a transfer may ask for, and how one is
//! brought to an end.
//!
//! The three types here are the parameters this crate does interpret, as
//! against the selectors it passes through. A `level`/`optname` pair names a
//! point in a namespace the service owns and can extend, so no enum written
//! here would stay complete. A transfer flag is not like that: the set is
//! closed, the C headers enumerate it, and the values are as much a part of
//! the interface as the command ids.
//!
//! # Why send and receive have separate flag sets
//!
//! Most `MSG_*` flags are valid in one direction only — `MSG_PEEK` and
//! `MSG_WAITALL` mean nothing on a send, `MSG_EOR` and `MSG_NOSIGNAL` nothing
//! on a receive. One shared set would accept every one of those on either
//! call, and the service would answer a request the caller never meant to
//! make. Two sets cost nothing and make the mistake a compile error.
//!
//! The output-only flags are deliberately absent. `MSG_TRUNC` and
//! `MSG_CTRUNC` are how a receive *reports* that it discarded data; they are
//! not something a caller asks for, so putting them in an input set would
//! invite sending a flag the service does not read.
//!
//! The `MSG_*` and `SHUT_*` values are FreeBSD's, which is the lineage the
//! service's socket interface follows.

bitflags::bitflags! {
    /// What a receive may ask for, beyond the default.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RecvFlags: i32 {
        /// Read out-of-band data rather than the normal stream.
        const OOB = 0x0000_0001;
        /// Return the data without consuming it, so the next receive sees it
        /// again.
        const PEEK = 0x0000_0002;
        /// Wait for the full buffer rather than returning at the first
        /// message. A signal or an error can still cut it short.
        const WAITALL = 0x0000_0040;
        /// Return rather than block if nothing has arrived, whatever the
        /// socket's own blocking mode.
        const DONTWAIT = 0x0000_0080;
        /// Return once one message has arrived rather than waiting for the
        /// whole vector. Read only by the multi-message receive.
        const WAITFORONE = 0x0008_0000;
    }
}

bitflags::bitflags! {
    /// What a send may ask for, beyond the default.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SendFlags: i32 {
        /// Send as out-of-band data.
        const OOB = 0x0000_0001;
        /// Bypass the routing tables and use the directly-attached interface.
        const DONTROUTE = 0x0000_0004;
        /// This send completes a record.
        const EOR = 0x0000_0008;
        /// Return rather than block if the send buffer is full, whatever the
        /// socket's own blocking mode.
        const DONTWAIT = 0x0000_0080;
        /// This send completes the connection.
        const EOF = 0x0000_0100;
        /// Report a closed peer as an error rather than raising `SIGPIPE`.
        const NOSIGNAL = 0x0002_0000;
    }
}

/// Which directions `shutdown` disables.
///
/// Mirrors `std::net::Shutdown`, which names the same three states — a socket
/// layer built on this crate can pass one through unchanged rather than
/// translating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Disable further receives; sends still work.
    Read,
    /// Disable further sends; receives still work.
    Write,
    /// Disable both.
    Both,
}

impl Shutdown {
    /// The `SHUT_*` value the service reads.
    pub(crate) const fn to_wire(self) -> i32 {
        match self {
            Self::Read => 0,
            Self::Write => 1,
            Self::Both => 2,
        }
    }
}

bitflags::bitflags! {
    /// A descriptor's status flags — the set `F_SETFL` replaces and `F_GETFL`
    /// reports.
    ///
    /// Only the flags that describe how a descriptor behaves are settable;
    /// the access mode and the open-time flags are fixed once the descriptor
    /// exists, so they have no place in a set built to be written back.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusFlags: i32 {
        /// Operations return rather than blocking when they would have to
        /// wait. What a socket layer sets to drive a socket non-blocking.
        const NONBLOCK = 0x0000_4000;
    }
}
