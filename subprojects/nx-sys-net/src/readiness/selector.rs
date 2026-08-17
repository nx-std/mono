//! The set of sockets a caller is waiting on, and the wait over it.

use alloc::vec::Vec;
use core::time::Duration;

use nx_service_bsd::{
    CommandError,
    PollFd,
    SocketFd,
};

use super::{
    Event,
    Events,
    Interest,
    Readiness,
    Token,
};
use crate::{
    session,
    socket::Socket,
};

/// The sockets a caller is waiting on, and what it wants to hear about each.
///
/// Built once and kept for as long as the caller is waiting on anything. Sockets are added and
/// removed as they come and go, and each [`Self::select`] reports whichever of them are ready.
///
/// ## What a registration holds, and what it does not
///
/// A registration keeps the socket's descriptor, not a borrow of the [`Socket`]. It has to: a
/// selector outlives every individual borrow, and a caller registers a socket and then goes on
/// using it. Storing the descriptor is what keeps the two independent.
///
/// The cost is that the selector cannot tell when a socket is closed. A [`Socket`] dropped while
/// still registered leaves behind a descriptor the service is free to reissue, and the next wait
/// asks about whatever now holds that number. **Deregister before dropping.** Nothing faults if a
/// caller does not: the wait simply answers about a socket the caller no longer has. That is why
/// this is a contract in prose rather than something the type system holds.
///
/// A registration that survives its socket by less than a reissue is reported with
/// [`Readiness::INVALID`], which a caller may treat as the signal to deregister it.
///
/// ## The wake channel is not one of the registrations
///
/// [`crate::Waker`] gives a selector a channel another thread can end its wait through, and that
/// channel is held apart from the registered sockets: it is not counted by [`Self::len`], cannot be
/// deregistered, and goes when the selector does. What it shares with a registration is the token
/// it reports under, which is why that token is one a caller reserves.
#[derive(Debug, Default)]
pub struct Selector {
    registered: Vec<Registration>,
    /// The receiving end of the wake channel, once a waker has been opened against this selector.
    wake: Option<WakeChannel>,
    /// The set the last wait sent, kept so a wait in a loop allocates nothing after the first.
    /// Rebuilt from `registered` on every wait rather than maintained alongside it, because an
    /// interest that changed between waits has to reach the service on the next one.
    scratch: Vec<PollFd>,
}

impl Selector {
    /// Builds a selector with nothing registered.
    pub const fn new() -> Self {
        Self {
            registered: Vec::new(),
            wake: None,
            scratch: Vec::new(),
        }
    }

    /// Adds `socket` to the set, to be reported under `token`.
    ///
    /// Takes the socket by reference, which is what proves it is open at the moment it is
    /// registered. What is kept afterwards is its descriptor; see the type's own documentation for
    /// what that means when the socket is closed.
    ///
    /// `token` must not already name another registration, which is the caller's to ensure and not
    /// checked here; see [`Token`] for why the rule sits there rather than in this call.
    ///
    /// # Errors
    ///
    /// [`RegisterError::AlreadyRegistered`] when the socket is already in the set. Re-registering
    /// under a second token would have the service answer about it twice, and a caller reading the
    /// second answer would act on it after having acted on the first;
    /// [`Self::reregister`] is how an interest or a token is changed.
    pub fn register(
        &mut self,
        socket: &Socket,
        token: Token,
        interest: Interest,
    ) -> Result<(), RegisterError> {
        self.register_fd(socket.service_fd(), token, interest)
    }

    /// [`Self::register`], addressing the socket by descriptor.
    ///
    /// The bookkeeping half, split from the public method so it can be exercised without a live
    /// socket: everything above the descriptor is pure, and everything below it needs a service.
    fn register_fd(
        &mut self,
        fd: SocketFd,
        token: Token,
        interest: Interest,
    ) -> Result<(), RegisterError> {
        if self.position_of(fd).is_some() {
            return Err(RegisterError::AlreadyRegistered);
        }

        self.registered.push(Registration {
            fd,
            token,
            interest,
        });
        Ok(())
    }

    /// Replaces what the set says about `socket`.
    ///
    /// The token it is reported under is one of the things this changes, under the same rule
    /// [`Self::register`] states: the new one must not already name another registration.
    ///
    /// # Errors
    ///
    /// [`ReregisterError::NotRegistered`] when the socket is not in the set. Adding it here instead
    /// would make a typo in a caller's bookkeeping look like a successful change.
    pub fn reregister(
        &mut self,
        socket: &Socket,
        token: Token,
        interest: Interest,
    ) -> Result<(), ReregisterError> {
        self.reregister_fd(socket.service_fd(), token, interest)
    }

    /// [`Self::reregister`], addressing the socket by descriptor.
    fn reregister_fd(
        &mut self,
        fd: SocketFd,
        token: Token,
        interest: Interest,
    ) -> Result<(), ReregisterError> {
        let Some(at) = self.position_of(fd) else {
            return Err(ReregisterError::NotRegistered);
        };

        self.registered[at].token = token;
        self.registered[at].interest = interest;
        Ok(())
    }

    /// Takes `socket` out of the set.
    ///
    /// What a caller does before dropping a socket it registered.
    ///
    /// # Errors
    ///
    /// [`DeregisterError::NotRegistered`] when the socket is not in the set.
    pub fn deregister(&mut self, socket: &Socket) -> Result<(), DeregisterError> {
        self.deregister_fd(socket.service_fd())
    }

    /// [`Self::deregister`], addressing the socket by descriptor.
    fn deregister_fd(&mut self, fd: SocketFd) -> Result<(), DeregisterError> {
        let Some(at) = self.position_of(fd) else {
            return Err(DeregisterError::NotRegistered);
        };

        // Order is what the answers are matched back by within a single wait, and each wait builds
        // its own set, so nothing carries across a removal. Shifting the tail keeps registrations
        // in the order they were made, which is the order events come back in.
        self.registered.remove(at);
        Ok(())
    }

    /// How many sockets are registered.
    #[inline]
    pub fn len(&self) -> usize {
        self.registered.len()
    }

    /// Whether nothing is registered.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.registered.is_empty()
    }

    /// Whether a wait on this selector can be ended by a [`crate::Waker`].
    ///
    /// False until one has been opened against it, which is also what makes opening a second one
    /// refusable rather than something that quietly unseats the first.
    #[inline]
    pub fn is_wakeable(&self) -> bool {
        self.wake.is_some()
    }

    /// Takes on the receiving end of a wake channel, to be reported under `token`.
    ///
    /// The caller checks [`Self::is_wakeable`] first: a channel attached over an existing one would
    /// close the socket the earlier waker sends to. Nothing here enforces that, because the one
    /// caller is [`crate::Waker::open`], which is where the refusal a user sees is raised.
    pub(super) fn attach_wake(&mut self, socket: Socket, token: Token) {
        self.wake = Some(WakeChannel { socket, token });
    }

    /// Waits until one of the registered sockets is ready, reporting them in `events`.
    ///
    /// `events` is cleared first, so what it holds afterwards is this wait's answer and nothing
    /// carried over. `None` for `timeout` waits indefinitely; a timeout that expires with nothing
    /// ready leaves `events` empty rather than failing.
    ///
    /// A wait with nothing registered and no wake channel reports nothing and returns at once. It
    /// is deliberately not a sleep: the service reads an empty set as one, and a caller waiting on
    /// no sockets did not ask to sleep. A selector with a [`crate::Waker`] against it always has
    /// the channel in its set, so its waits block whether or not anything is registered, which is
    /// what an event loop with no connections yet needs.
    ///
    /// # Errors
    ///
    /// [`SelectError::NotConnected`] when the driver is not initialized, and
    /// [`SelectError::Service`] when the service refused the wait, which it does for a set larger
    /// than it will accept and for a wait broken off before it finished. A socket the service does
    /// not recognise is not a failure of the call: it is reported with [`Readiness::INVALID`] and
    /// the rest are waited on as asked.
    pub fn select(
        &mut self,
        events: &mut Events,
        timeout: Option<Duration>,
    ) -> Result<(), SelectError> {
        events.clear();

        if self.registered.is_empty() && self.wake.is_none() {
            return Ok(());
        }

        // The wake channel goes in first when there is one, so it holds the first answer and the
        // registrations follow in the order they were made.
        self.scratch.clear();
        if let Some(wake) = &self.wake {
            self.scratch.push(PollFd::new(
                wake.socket.service_fd(),
                Interest::READABLE.into(),
            ));
        }
        self.scratch.extend(
            self.registered
                .iter()
                .map(|reg| PollFd::new(reg.fd, reg.interest.into())),
        );

        session::with_service(|svc| svc.poll(&mut self.scratch, timeout))
            .map_err(|_| SelectError::NotConnected)?
            .map_err(SelectError::Service)?;

        let mut answers = self.scratch.as_slice();

        if let Some(wake) = &self.wake
            && let Some((answered, registered_answers)) = answers.split_first()
        {
            let readiness = Readiness::from(answered.revents());
            if !readiness.is_empty() {
                // Before the caller is told, so that a wake collected here does not also wake the
                // wait after this one. The channel is level-triggered like every other socket in
                // the set: what is left in it is ready again immediately.
                drain(&wake.socket);
                events.push(Event::new(wake.token, readiness));
            }
            answers = registered_answers;
        }

        // The set was built from the registrations in order and the service answers in place, so
        // position is what pairs an answer with the registration that asked for it.
        for (registration, answered) in self.registered.iter().zip(answers) {
            let readiness = Readiness::from(answered.revents());
            if !readiness.is_empty() {
                events.push(Event::new(registration.token, readiness));
            }
        }

        Ok(())
    }

    /// Where `fd` sits in the set, if it is in it.
    fn position_of(&self, fd: SocketFd) -> Option<usize> {
        self.registered.iter().position(|reg| reg.fd == fd)
    }
}

/// One socket in the set, and what the caller asked about it.
#[derive(Debug)]
struct Registration {
    /// Names the socket. Not an owner: the [`Socket`] the caller holds is the only closer, and this
    /// is the descriptor rebuilt into a wire entry on each wait.
    fd: SocketFd,
    token: Token,
    interest: Interest,
}

/// The receiving end of a wake channel, and the token a wake is reported under.
///
/// Unlike a [`Registration`] this owns its socket, because nothing else holds it: the waker keeps
/// the sending end and a caller never sees either.
#[derive(Debug)]
struct WakeChannel {
    socket: Socket,
    token: Token,
}

/// Errors returned by [`Selector::register`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// The socket is already in the set
    ///
    /// The set is unchanged, and the registration already there is untouched.
    #[error("the socket is already registered")]
    AlreadyRegistered,
}

/// Errors returned by [`Selector::reregister`].
#[derive(Debug, thiserror::Error)]
pub enum ReregisterError {
    /// The socket is not in the set
    ///
    /// The set is unchanged.
    #[error("the socket is not registered")]
    NotRegistered,
}

/// Errors returned by [`Selector::deregister`].
#[derive(Debug, thiserror::Error)]
pub enum DeregisterError {
    /// The socket is not in the set
    ///
    /// The set is unchanged.
    #[error("the socket is not registered")]
    NotRegistered,
}

/// Errors returned by [`Selector::select`].
#[derive(Debug, thiserror::Error)]
pub enum SelectError {
    /// The socket driver is not initialized
    ///
    /// Occurs when the wait is issued before [`crate::initialize`] or after [`crate::exit`].
    /// Nothing was sent, and the events buffer is left empty.
    #[error("The socket service is not connected")]
    NotConnected,

    /// The service refused the wait
    ///
    /// The events buffer is left empty, so no socket reads as ready. Safe to retry: a refused wait
    /// changes nothing about the sockets it named, and the set is still registered.
    #[error("The socket service refused the wait")]
    Service(#[source] CommandError),
}

/// Empties the wake channel of everything that has arrived in it.
///
/// A wake is one datagram, so a burst of them is a burst of datagrams, and reading one would leave
/// the rest to report the channel ready again on the next wait. This reads until there are none
/// left, which on the non-blocking socket the waker opened is the receive that finds the channel
/// empty.
///
/// That receive's failure is what ends the loop and is discarded: an empty channel and a channel
/// that can no longer be read mean the same thing here, which is that there is nothing more to take
/// out of it.
fn drain(socket: &Socket) {
    // A wake carries one byte and nothing reads what it carries. A datagram longer than this is
    // truncated to it and discarded, which is the right handling for something nobody sent.
    let mut discarded = [0u8; 1];
    while socket.recv(&mut discarded).is_ok() {}
}

#[cfg(test)]
mod tests {
    use nx_service_bsd::SocketFd;

    use super::{
        Interest,
        Selector,
        Socket,
        Token,
    };

    /// Names a socket for the bookkeeping to hold.
    ///
    /// Never sent anywhere: these tests exercise the half of the selector that sits above the
    /// descriptor, so the number only has to be distinct and non-negative.
    fn fd(raw: i32) -> SocketFd {
        SocketFd::from_raw_unchecked(raw)
    }

    #[test]
    fn register_fd_with_an_unregistered_socket_adds_it() {
        //* Given
        let mut selector = Selector::new();

        //* When
        let result = selector.register_fd(fd(3), Token::new(1), Interest::READABLE);

        //* Then
        assert!(
            result.is_ok(),
            "registering an absent socket should succeed"
        );
        assert_eq!(
            selector.len(),
            1,
            "the set should hold the one registration"
        );
    }

    #[test]
    fn register_fd_with_an_already_registered_socket_fails() {
        //* Given
        let mut selector = Selector::new();
        selector
            .register_fd(fd(3), Token::new(1), Interest::READABLE)
            .expect("the first registration should succeed");

        //* When
        let result = selector.register_fd(fd(3), Token::new(2), Interest::WRITABLE);

        //* Then
        assert!(
            result.is_err(),
            "a second registration of one socket should be refused"
        );
        assert_eq!(
            selector.len(),
            1,
            "the refused registration must not have been added"
        );
    }

    #[test]
    fn reregister_fd_with_a_registered_socket_replaces_its_token_and_interest() {
        //* Given
        let mut selector = Selector::new();
        selector
            .register_fd(fd(3), Token::new(1), Interest::READABLE)
            .expect("the registration should succeed");

        //* When
        let result = selector.reregister_fd(fd(3), Token::new(9), Interest::WRITABLE);

        //* Then
        assert!(
            result.is_ok(),
            "re-registering a present socket should succeed"
        );
        assert_eq!(
            selector.len(),
            1,
            "re-registering must not add a second entry"
        );
        let registration = selector
            .registered
            .first()
            .expect("the set should still hold the registration");
        assert_eq!(
            registration.token,
            Token::new(9),
            "the token should be replaced"
        );
        assert_eq!(
            registration.interest,
            Interest::WRITABLE,
            "the interest should be replaced"
        );
    }

    #[test]
    fn reregister_fd_with_an_unregistered_socket_fails() {
        //* Given
        let mut selector = Selector::new();

        //* When
        let result = selector.reregister_fd(fd(3), Token::new(1), Interest::READABLE);

        //* Then
        assert!(
            result.is_err(),
            "re-registering a socket that was never registered should be refused"
        );
        assert!(
            selector.is_empty(),
            "a refused re-registration must not add the socket"
        );
    }

    #[test]
    fn deregister_fd_with_a_registered_socket_removes_it() {
        //* Given
        let mut selector = Selector::new();
        selector
            .register_fd(fd(3), Token::new(1), Interest::READABLE)
            .expect("the registration should succeed");

        //* When
        let result = selector.deregister_fd(fd(3));

        //* Then
        assert!(
            result.is_ok(),
            "deregistering a present socket should succeed"
        );
        assert!(selector.is_empty(), "the set should be empty again");
    }

    #[test]
    fn deregister_fd_with_an_unregistered_socket_fails() {
        //* Given
        let mut selector = Selector::new();

        //* When
        let result = selector.deregister_fd(fd(3));

        //* Then
        assert!(
            result.is_err(),
            "deregistering a socket that was never registered should be refused"
        );
    }

    #[test]
    fn deregister_fd_on_a_middle_socket_keeps_the_rest_in_order() {
        //* Given
        // Three registrations, because order is what pairs an answer with the registration that
        // asked for it and only a removal from the middle can disturb it.
        let mut selector = Selector::new();
        for (raw, token) in [(3, 1), (4, 2), (5, 3)] {
            selector
                .register_fd(fd(raw), Token::new(token), Interest::READABLE)
                .expect("each registration should succeed");
        }

        //* When
        let result = selector.deregister_fd(fd(4));

        //* Then
        assert!(
            result.is_ok(),
            "deregistering the middle socket should succeed"
        );
        let tokens: alloc::vec::Vec<Token> =
            selector.registered.iter().map(|reg| reg.token).collect();
        assert_eq!(
            tokens,
            alloc::vec![Token::new(1), Token::new(3)],
            "the survivors should keep the order they were registered in"
        );
    }

    #[test]
    fn is_wakeable_on_a_fresh_selector_is_false() {
        //* Given
        let selector = Selector::new();

        //* When
        let wakeable = selector.is_wakeable();

        //* Then
        assert!(
            !wakeable,
            "a selector nothing has been attached to must not claim a wake channel"
        );
    }

    #[test]
    fn attach_wake_makes_the_selector_wakeable_without_registering_anything() {
        //* Given
        let mut selector = Selector::new();

        //* When
        // SAFETY: the descriptor names nothing, and the socket is never sent a command; the
        // close its drop attempts is against a driver these tests never connect.
        selector.attach_wake(Socket::from_raw_unchecked(fd(3)), Token::new(1));

        //* Then
        assert!(
            selector.is_wakeable(),
            "an attached channel should be what makes a selector wakeable"
        );
        assert!(
            selector.is_empty(),
            "the wake channel is not a registration and must not be counted as one"
        );
    }

    #[test]
    fn register_fd_after_a_deregistration_reuses_the_freed_descriptor() {
        //* Given
        // The service reissues descriptors, so a number that was registered and removed can come
        // back naming a different socket. The set must accept it rather than remember the old one.
        let mut selector = Selector::new();
        selector
            .register_fd(fd(3), Token::new(1), Interest::READABLE)
            .expect("the registration should succeed");
        selector
            .deregister_fd(fd(3))
            .expect("the deregistration should succeed");

        //* When
        let result = selector.register_fd(fd(3), Token::new(2), Interest::WRITABLE);

        //* Then
        assert!(
            result.is_ok(),
            "a descriptor the set no longer holds should register like any other"
        );
        assert_eq!(
            selector.len(),
            1,
            "the set should hold the new registration"
        );
    }
}
