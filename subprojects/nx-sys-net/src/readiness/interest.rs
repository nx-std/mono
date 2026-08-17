//! What a caller asks to be told about a socket.

use nx_service_bsd::PollEvents;

bitflags::bitflags! {
    /// What a caller asks to be told about.
    ///
    /// Three of the six conditions a wait can report, and deliberately not the other three: a
    /// socket that failed, one whose peer hung up and one the service does not recognise are
    /// reported whether or not anybody asked, so a type that let a caller ask for them would offer
    /// a request that changes nothing. [`Readiness`](super::Readiness) is the type that can carry
    /// all six.
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

impl From<Interest> for PollEvents {
    fn from(interest: Interest) -> Self {
        let mut events = Self::empty();
        events.set(Self::IN, interest.contains(Interest::READABLE));
        events.set(Self::OUT, interest.contains(Interest::WRITABLE));
        events.set(Self::PRI, interest.contains(Interest::PRIORITY));
        events
    }
}

#[cfg(test)]
mod tests {
    use nx_service_bsd::PollEvents;

    use super::Interest;

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
}
