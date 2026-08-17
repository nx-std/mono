//! Waiting until sockets are ready.
//!
//! Every other operation on a [`Socket`] acts on one socket and returns when that socket is done.
//! This one is the exception: it takes a set of them, blocks until any is ready, and reports which.
//! That is what an event loop is built out of, and it is the only operation here that a caller
//! cannot assemble from the others.
//!
//! ## There is one wait, and it is this one
//!
//! The service offers `Poll` and `Select`, and no registration interface: nothing here corresponds
//! to `epoll` or `kqueue`, where a program declares its interest once and collects events many
//! times. Each wait carries the whole set. A caller watching the same sockets repeatedly keeps its
//! own [`Watch`] array and passes it again, which costs one copy of the set per wait and is the
//! shape the service imposes rather than one chosen here.
//!
//! `Select` is not wrapped at all. It answers in bitmaps keyed by descriptor number, and the
//! numbers a bitmap would be keyed by are the service's own, which no caller of this crate holds.
//! `Poll` carries one entry per socket, so the correspondence between what was asked and what was
//! answered survives without a caller having to reconstruct it.

use alloc::vec::Vec;
use core::time::Duration;

use nx_service_bsd::{
    CommandError,
    PollEvents,
    PollFd,
};

use crate::{
    session,
    socket::Socket,
};

/// Waits until one of `watches` is ready, reporting how many are.
///
/// Each [`Watch`] is written back with what the service reported, readable through
/// [`Watch::readiness`]. `None` for `timeout` waits indefinitely; a timeout that expires with
/// nothing ready is a count of zero rather than a failure.
///
/// Every watch is cleared before the wait, so a readiness left over from a previous call is never
/// mistaken for an answer this one produced.
///
/// # Errors
///
/// [`PollError::NotConnected`] when the driver is not initialized, and [`PollError::Service`] when
/// the service refused the wait, which it does for a set larger than it will accept and for a wait
/// broken off before it finished. A socket the service does not recognise is not a failure of the
/// call: that watch comes back carrying [`Readiness::INVALID`] and the rest are waited on as asked.
pub fn poll(watches: &mut [Watch<'_>], timeout: Option<Duration>) -> Result<usize, PollError> {
    for watch in watches.iter_mut() {
        watch.readiness = Readiness::empty();
    }

    if watches.is_empty() {
        // Nothing was asked about, so nothing became ready. The command is deliberately not sent:
        // the service reads an empty array as a plain sleep, and a caller watching no sockets did
        // not ask to sleep.
        return Ok(0);
    }

    let mut entries: Vec<PollFd> = watches
        .iter()
        .map(|watch| PollFd::new(watch.socket.service_fd(), watch.interest.into()))
        .collect();

    let ready = session::with_service(|svc| svc.poll(&mut entries, timeout))
        .map_err(|_| PollError::NotConnected)?
        .map_err(PollError::Service)?;

    // The entries were built from the watches in order and the service answers in place, so
    // position is what pairs an answer with the watch that asked for it.
    for (watch, entry) in watches.iter_mut().zip(entries.iter()) {
        watch.readiness = entry.revents().into();
    }

    Ok(ready)
}

/// Errors returned by [`poll`].
#[derive(Debug, thiserror::Error)]
pub enum PollError {
    /// The socket driver is not initialized
    ///
    /// Occurs when the wait is issued before [`crate::initialize`] or after [`crate::exit`].
    /// Nothing was sent and no watch was written.
    #[error("The socket service is not connected")]
    NotConnected,

    /// The service refused the wait
    ///
    /// The watches are left cleared, so no socket reads as ready. Safe to retry: a refused wait
    /// changes nothing about the sockets it named.
    #[error("The socket service refused the wait")]
    Service(#[source] CommandError),
}

/// One socket, what a caller wants to hear about it, and what came back.
///
/// Borrows the socket rather than naming it by number, which is what makes a watch on a closed
/// socket unspellable: the [`Socket`] cannot be dropped while a watch refers to it.
#[derive(Debug)]
pub struct Watch<'s> {
    socket: &'s Socket,
    interest: Interest,
    readiness: Readiness,
}

impl<'s> Watch<'s> {
    /// Watches `socket` for `interest`.
    ///
    /// Reports nothing until it has been through [`poll`].
    #[inline]
    pub const fn new(socket: &'s Socket, interest: Interest) -> Self {
        Self {
            socket,
            interest,
            readiness: Readiness::empty(),
        }
    }

    /// The socket this watch is about.
    #[inline]
    pub const fn socket(&self) -> &'s Socket {
        self.socket
    }

    /// What this watch asked about.
    #[inline]
    pub const fn interest(&self) -> Interest {
        self.interest
    }

    /// What the last [`poll`] reported about the socket.
    ///
    /// Empty before the first wait, and empty after one the socket was not ready for.
    #[inline]
    pub const fn readiness(&self) -> Readiness {
        self.readiness
    }

    /// Whether the last [`poll`] reported anything at all.
    ///
    /// True for a socket in trouble as well as a ready one: a hangup and an error are both things
    /// the caller was told, and a caller that skipped them would wait on that socket forever.
    #[inline]
    pub const fn is_ready(&self) -> bool {
        !self.readiness.is_empty()
    }
}

bitflags::bitflags! {
    /// What a caller asks to be told about.
    ///
    /// Three of the six conditions a wait can report, and deliberately not the other three: a
    /// socket that failed, one whose peer hung up and one the service does not recognise are
    /// reported whether or not anybody asked, so a type that let a caller ask for them would offer
    /// a request that changes nothing. [`Readiness`] is the type that can carry all six.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Interest: u8 {
        /// Tell me when a read will not block.
        const READABLE = 1 << 0;
        /// Tell me when a write will not block.
        const WRITABLE = 1 << 1;
        /// Tell me when out-of-band data arrives.
        const PRIORITY = 1 << 2;
    }
}

bitflags::bitflags! {
    /// What a wait reported about a socket.
    ///
    /// The last three arrive unasked, so a caller that watched only for [`Interest::READABLE`] can
    /// still be handed [`Self::HANGUP`]. Treating an unasked-for condition as nothing is the way an
    /// event loop hangs: the socket never becomes readable, and the wait that would have said why
    /// is the one being ignored.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Readiness: u8 {
        /// A read will not block. It may still read zero bytes, which is how a stream reports its
        /// peer is done sending.
        const READABLE = 1 << 0;
        /// A write will not block.
        const WRITABLE = 1 << 1;
        /// Out-of-band data is waiting.
        const PRIORITY = 1 << 2;
        /// The socket failed, and the error is waiting to be read off it.
        const ERROR = 1 << 3;
        /// The peer closed its end.
        const HANGUP = 1 << 4;
        /// The service does not recognise the socket.
        const INVALID = 1 << 5;
    }
}

impl From<Interest> for PollEvents {
    fn from(interest: Interest) -> Self {
        let mut events = Self::empty();
        events.set(Self::IN, interest.contains(Interest::READABLE));
        events.set(Self::OUT, interest.contains(Interest::WRITABLE));
        events.set(Self::PRI, interest.contains(Interest::PRIORITY));
        events
    }
}

impl From<PollEvents> for Readiness {
    /// A condition outside the six named here is dropped rather than carried, because there is no
    /// variant to carry it in and nothing a caller could do with one it cannot name. The C surface
    /// is unaffected: it hands the service's answer back byte for byte and never comes through
    /// this conversion.
    fn from(events: PollEvents) -> Self {
        let mut readiness = Self::empty();
        readiness.set(Self::READABLE, events.contains(PollEvents::IN));
        readiness.set(Self::WRITABLE, events.contains(PollEvents::OUT));
        readiness.set(Self::PRIORITY, events.contains(PollEvents::PRI));
        readiness.set(Self::ERROR, events.contains(PollEvents::ERR));
        readiness.set(Self::HANGUP, events.contains(PollEvents::HUP));
        readiness.set(Self::INVALID, events.contains(PollEvents::NVAL));
        readiness
    }
}

#[cfg(test)]
mod tests {
    use nx_service_bsd::PollEvents;

    use super::{
        Interest,
        Readiness,
    };

    #[test]
    fn poll_events_from_interest_with_every_interest_names_three_conditions() {
        //* Given
        let interest = Interest::READABLE | Interest::WRITABLE | Interest::PRIORITY;

        //* When
        let events = PollEvents::from(interest);

        //* Then
        assert_eq!(
            events,
            PollEvents::IN | PollEvents::OUT | PollEvents::PRI,
            "each interest names one condition the wire carries"
        );
    }

    #[test]
    fn poll_events_from_interest_with_no_interest_asks_for_nothing() {
        //* Given
        let interest = Interest::empty();

        //* When
        let events = PollEvents::from(interest);

        //* Then
        assert!(
            events.is_empty(),
            "a watch that asks for nothing must not ask for something"
        );
    }

    #[test]
    fn readiness_from_poll_events_with_an_unasked_condition_keeps_it() {
        //* Given
        // A hangup on a socket that was only ever watched for readability.
        let events = PollEvents::IN | PollEvents::HUP;

        //* When
        let readiness = Readiness::from(events);

        //* Then
        assert_eq!(
            readiness,
            Readiness::READABLE | Readiness::HANGUP,
            "a condition reported unasked reaches the caller"
        );
    }

    #[test]
    fn readiness_from_poll_events_with_a_condition_this_layer_cannot_name_drops_it() {
        //* Given
        // A bit past the six the interface documents, which the wire type retains.
        let events = PollEvents::IN | PollEvents::from_bits_retain(0x0100);

        //* When
        let readiness = Readiness::from(events);

        //* Then
        assert_eq!(
            readiness,
            Readiness::READABLE,
            "only the conditions this layer names survive the conversion"
        );
    }

    #[test]
    fn readiness_from_poll_events_with_nothing_reported_is_empty() {
        //* Given
        let events = PollEvents::empty();

        //* When
        let readiness = Readiness::from(events);

        //* Then
        assert!(
            readiness.is_empty(),
            "a socket the service said nothing about is not ready"
        );
    }
}
