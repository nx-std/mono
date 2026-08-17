//! What a wait reported, and the name the caller gets it back under.
//!
//! A wait answers about sockets, and a caller thinks in terms of whatever it attached to them: a
//! connection, a queue, a slot in its own table. [`Token`] is that association, chosen by the
//! caller at registration and handed back with every [`Event`]. It is what lets the answer be
//! routed without the caller having to search its own state for the socket the wait named.

use alloc::vec::Vec;

use nx_service_bsd::PollEvents;

/// The name a caller registered a socket under.
///
/// Opaque here and meaningful to the caller alone: this layer never interprets one, it only hands
/// back the one that was registered. A caller that keeps its connections in a slab uses the slab
/// index; one that keeps a handful uses a constant apiece.
///
/// # One registration per token
///
/// A token must name one registration at a time. An [`Event`] carries the token and what the
/// socket reported, and never the socket itself, so two sockets sharing a token produce two events
/// a caller cannot tell apart, and the socket it guesses wrong about is the one it then fails to
/// service.
///
/// This is not checked. The check would be a scan on every registration for a collision the caller
/// is the only one able to cause, and the callers this layer is built for cannot cause it: a caller
/// minting tokens from a slab index, or from the address of the state it keeps per socket, gets
/// uniqueness from the allocator rather than from a rule. Enforcing it here would also add a
/// failure that has no counterpart in the interfaces built on top of this one.
///
/// A caller minting tokens by hand reserves the constants it uses for anything that is not a
/// socket, such as a wakeup or a listener, the way it would reserve any other name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(usize);

impl Token {
    /// Names a registration.
    #[inline]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// The value this token was built from.
    #[inline]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// One socket the wait had something to say about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    token: Token,
    readiness: Readiness,
}

impl Event {
    /// Builds an event reporting `readiness` for the socket registered under `token`.
    pub(crate) const fn new(token: Token, readiness: Readiness) -> Self {
        Self { token, readiness }
    }

    /// The token the socket this is about was registered under.
    #[inline]
    pub const fn token(&self) -> Token {
        self.token
    }

    /// What the wait reported about it.
    #[inline]
    pub const fn readiness(&self) -> Readiness {
        self.readiness
    }
}

/// The events one wait produced.
///
/// Reused across waits rather than returned by each: the buffer a caller hands in is cleared and
/// refilled, so a loop that waits forever allocates once. This is why a wait takes it by reference
/// instead of returning a collection.
#[derive(Debug, Default)]
pub struct Events {
    inner: Vec<Event>,
}

impl Events {
    /// Builds a buffer that can hold `capacity` events before it has to grow.
    ///
    /// The capacity is not a limit on what a wait may report: a wait that finds more sockets ready
    /// grows the buffer rather than dropping the excess, since an event nobody is told about is a
    /// socket the caller waits on forever.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// The events the last wait reported, in the order the registrations were made.
    pub fn iter(&self) -> core::slice::Iter<'_, Event> {
        self.inner.iter()
    }

    /// How many events the last wait reported.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the last wait reported nothing.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Discards what the last wait reported, keeping the buffer's capacity.
    pub(crate) fn clear(&mut self) {
        self.inner.clear();
    }

    /// Records one event.
    pub(crate) fn push(&mut self, event: Event) {
        self.inner.push(event);
    }
}

impl<'e> IntoIterator for &'e Events {
    type Item = &'e Event;
    type IntoIter = core::slice::Iter<'e, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

bitflags::bitflags! {
    /// What a wait reported about a socket.
    ///
    /// The last three arrive unasked, so a caller that watched only for
    /// [`Interest::READABLE`](super::Interest::READABLE) can still be handed [`Self::HANGUP`].
    /// Treating an unasked-for condition as nothing is the way an event loop hangs: the socket
    /// never becomes readable, and the wait that would have said why is the one being ignored.
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
        Event,
        Events,
        Readiness,
        Token,
    };

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

    #[test]
    fn clear_after_a_wait_reported_events_empties_the_buffer() {
        //* Given
        let mut events = Events::with_capacity(4);
        events.push(Event::new(Token::new(7), Readiness::READABLE));

        //* When
        events.clear();

        //* Then
        assert!(
            events.is_empty(),
            "a cleared buffer must not still report the previous wait"
        );
    }

    #[test]
    fn iter_over_recorded_events_yields_them_in_order() {
        //* Given
        let mut events = Events::with_capacity(2);
        events.push(Event::new(Token::new(1), Readiness::READABLE));
        events.push(Event::new(Token::new(2), Readiness::WRITABLE));

        //* When
        let tokens: alloc::vec::Vec<Token> = events.iter().map(Event::token).collect();

        //* Then
        assert_eq!(
            tokens,
            alloc::vec![Token::new(1), Token::new(2)],
            "events come back in the order they were recorded"
        );
    }
}
