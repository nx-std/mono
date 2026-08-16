//! What a caller asks about a socket, and what the service answers.
//!
//! The `Poll` command takes an array of entries rather than a count and a set of bitmaps, and it
//! writes its answer back into the same array. [`PollFd`] is one of those entries, laid out the way
//! the service reads it.

use crate::fd::SocketFd;

bitflags::bitflags! {
    /// The conditions a poll entry carries.
    ///
    /// One set rather than two, because the wire has one: a request and its answer are the same
    /// sixteen bits, and an entry carries both. Which of them a caller may usefully ask for is a
    /// distinction this layer does not draw; the layer that presents sockets to a program does.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PollEvents: i16 {
        /// There is data to read.
        const IN = 0x0001;
        /// There is out-of-band data to read.
        const PRI = 0x0002;
        /// Writing will not block.
        const OUT = 0x0004;
        /// The socket failed. Reported whether or not it was asked for.
        const ERR = 0x0008;
        /// The peer closed its end. Reported whether or not it was asked for.
        const HUP = 0x0010;
        /// The descriptor names no socket. Reported whether or not it was asked for.
        const NVAL = 0x0020;
    }
}

/// One socket's readiness request, and the answer written back into it.
///
/// The descriptor is write-only: the field goes out on the wire and nothing reads it back, because
/// a caller that built the array already knows which socket each position is about, and the
/// service does not renumber them. Keeping it private is what lets the struct be reconstituted
/// from arbitrary bytes, which is what the service writing into the array requires.
#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
pub struct PollFd {
    /// The socket, as the service numbers it.
    fd: i32,
    /// What the caller asked about.
    events: i16,
    /// What the service reported, written back by the command.
    revents: i16,
}

impl PollFd {
    /// Asks about `socket`.
    ///
    /// The entry starts with nothing reported. [`Self::revents`] answers once the command that
    /// took the array has returned.
    #[inline]
    pub const fn new(socket: SocketFd, events: PollEvents) -> Self {
        Self {
            fd: socket.to_raw(),
            events: events.bits(),
            revents: 0,
        }
    }

    /// What this entry asked about.
    #[inline]
    pub const fn events(&self) -> PollEvents {
        PollEvents::from_bits_retain(self.events)
    }

    /// What the service reported about the socket.
    ///
    /// Empty until the command has run, and empty afterwards for a socket that did not become
    /// ready. A bit outside the six named above is retained rather than dropped: the service
    /// descends from a BSD stack, whose headers define conditions past the POSIX set, and a caller
    /// handing this answer back to C compares it against those headers rather than against this
    /// type.
    #[inline]
    pub const fn revents(&self) -> PollEvents {
        PollEvents::from_bits_retain(self.revents)
    }
}
