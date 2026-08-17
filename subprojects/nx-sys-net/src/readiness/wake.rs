//! Ending a wait from a thread that is not in it.
//!
//! A wait blocks until one of the sockets it was given is ready, and a thread sitting in one cannot
//! be reached: the [`Selector`] is borrowed for the duration, so the thread holding it is the only
//! thread that can touch the set. Everything an event loop is asked to do from elsewhere -- a task
//! scheduled, a shutdown requested, a socket to start watching -- arrives while that wait is in
//! progress and waits for it to end on its own.
//!
//! [`Waker`] is the way to end it. It is a handle with no borrow of the selector in it, so it moves
//! to another thread, and waking is the one thing it does.
//!
//! ## Two sockets, because there is nothing else to build it out of
//!
//! Elsewhere this would be a pipe or an event descriptor: a thing whose whole purpose is to become
//! readable when somebody says so. The service offers neither. What it offers is sockets, so the
//! channel is a pair of them talking over loopback, and the wait notices it the same way it notices
//! any other socket becoming readable.
//!
//! Datagrams rather than a stream, because a datagram channel cannot half-close, cannot be refused
//! for want of an accept, and drops what it cannot hold instead of blocking the sender. That last
//! one is what makes [`Waker::wake`] total: see its documentation for why a dropped wake is not a
//! missed one.
//!
//! ## Which half lives where
//!
//! The receiving socket goes into the selector, and the sending socket is what this type holds. The
//! split is what keeps the drain out of the caller's hands: the wait knows when the channel has
//! been read as ready, so the wait is what empties it, and a caller that never learns the channel
//! exists cannot forget to.

use core::net::{
    Ipv4Addr,
    SocketAddr,
    SocketAddrV4,
};

use nx_service_bsd::SockType;

use super::{
    Selector,
    Token,
};
use crate::socket::{
    Error as SocketError,
    Socket,
};

/// Ends a wait on the [`Selector`] it was opened against.
///
/// Holds no borrow of the selector, so it moves to whichever thread needs to do the waking, and is
/// shared between several by putting it in an `Arc`.
///
/// ## It does not outlive its selector
///
/// The channel has two ends and this is one of them. A selector dropped while a waker still exists
/// takes the receiving end with it, and [`Self::wake`] afterwards sends to a port nothing holds:
/// what that reports is the stack's business and no wait is woken by it either way. Nothing faults,
/// and nothing is woken.
#[derive(Debug)]
pub struct Waker {
    /// Sends the datagram a wait notices. Owned: dropping this closes the socket, and the receiving
    /// end is owned just as singly by the selector.
    sender: Socket,
}

impl Waker {
    /// One byte, because the wait reads only that something arrived.
    ///
    /// The channel carries no information: every wake means the same thing, and a caller that needs
    /// to say more than "look again" says it through its own state and lets the wake be the nudge
    /// to go and read it.
    const WAKE: [u8; 1] = [1];

    /// Opens a wake channel and gives `selector` the receiving end of it.
    ///
    /// From here a wait on `selector` blocks until a socket is ready **or** [`Self::wake`] is
    /// called, and a wake is reported as [`Readiness::READABLE`](super::Readiness::READABLE) under
    /// `token`.
    ///
    /// `token` names the channel for as long as it is attached, so it is one of the names a caller
    /// reserves rather than one it hands to a socket; see [`Token`] for the rule and why it is not
    /// checked.
    ///
    /// # Errors
    ///
    /// [`OpenWakerError::AlreadyWakeable`] when `selector` already has a channel, which is refused
    /// rather than replaced: replacing it would leave the earlier waker sending into a socket
    /// nothing is waiting on, and a wake that reaches nobody is indistinguishable from one that was
    /// never asked for. Nothing is opened, and the existing channel is untouched.
    ///
    /// [`OpenWakerError::Receiver`] and [`OpenWakerError::Sender`] when the service refused one of the
    /// sockets the channel is made of. `selector` is left without a channel, and both sockets are
    /// closed on the way out.
    pub fn open(selector: &mut Selector, token: Token) -> Result<Self, OpenWakerError> {
        if selector.is_wakeable() {
            return Err(OpenWakerError::AlreadyWakeable);
        }

        // Port zero, because which port the channel gets does not matter: both ends are in this
        // process and the sender is told the assigned one below. Asking for a fixed port would be
        // asking for the one collision this design can suffer.
        let loopback = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));

        let receiver =
            Socket::open(&loopback, SockType::Dgram).map_err(OpenWakerError::Receiver)?;
        receiver.bind(&loopback).map_err(OpenWakerError::Receiver)?;

        // The wait empties the channel by receiving until there is nothing left, and on a blocking
        // socket the receive that finds nothing left is where it would stop and stay.
        receiver
            .set_nonblocking(true)
            .map_err(OpenWakerError::Receiver)?;

        // Which port the bind was given, which is the one thing the sender needs and the one thing
        // that could not be known before it.
        let assigned = receiver.local_addr().map_err(OpenWakerError::Receiver)?;

        let sender = Socket::open(&assigned, SockType::Dgram).map_err(OpenWakerError::Sender)?;
        sender.connect(&assigned).map_err(OpenWakerError::Sender)?;

        selector.attach_wake(receiver, token);
        Ok(Self { sender })
    }

    /// Ends the wait in progress on the selector, or the next one to start.
    ///
    /// Safe to call from any thread and at any time, including when no wait is running: the
    /// datagram waits in the channel and the next wait returns on it at once. Several wakes with no
    /// wait between them are one wake, since the wait that finds them drains the channel before
    /// reporting.
    ///
    /// # Errors
    ///
    /// [`WakeError`] when the service refused the send for a reason other than a full channel. A
    /// full channel is reported as success and not retried: it is full of wakes nobody has
    /// collected yet, so the wait this one would have caused is already going to happen, and adding
    /// a second datagram to a queue that has one in it changes nothing.
    pub fn wake(&self) -> Result<(), WakeError> {
        match self.sender.send(&Self::WAKE) {
            // The count is not checked: a datagram is sent whole or not at all, so there is no
            // short send to handle the way there is on a stream.
            Ok(_) => Ok(()),
            Err(err) if err.is_would_block() => Ok(()),
            Err(err) => Err(WakeError(err)),
        }
    }
}

/// Errors returned by [`Waker::open`].
#[derive(Debug, thiserror::Error)]
pub enum OpenWakerError {
    /// The selector already has a wake channel
    ///
    /// Nothing was opened and the selector is unchanged, so the waker that owns the existing
    /// channel goes on working.
    #[error("the selector already has a wake channel")]
    AlreadyWakeable,

    /// The receiving end could not be opened
    ///
    /// Carries whichever of the four commands it takes to make a bound, non-blocking socket the
    /// service refused.
    #[error("the receiving end of the wake channel could not be opened")]
    Receiver(#[source] SocketError),

    /// The sending end could not be opened
    ///
    /// The receiving end was opened and is closed again before this is returned.
    #[error("the sending end of the wake channel could not be opened")]
    Sender(#[source] SocketError),
}

/// Errors returned by [`Waker::wake`].
///
/// One way to fail rather than a choice of them: the send either reached the channel or it did not,
/// and which command refused is in the source. No wait was woken by a call that returns this, and a
/// wait already in progress stays in progress until something it was watching is ready or its
/// timeout runs out.
#[derive(Debug, thiserror::Error)]
#[error("the wake could not be sent")]
pub struct WakeError(#[source] pub SocketError);
