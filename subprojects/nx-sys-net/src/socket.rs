//! One open socket, and the obligation to close it.
//!
//! [`nx_service_bsd::BsdSockFd`] is a number the service issues and expects back exactly once, and
//! it is `Copy` — deliberately, so that the command wrappers can take one freely. That leaves
//! somebody to hold the close obligation, and [`nx_service_bsd`] says so outright: taking the
//! descriptor by value in its close command is what lets the layer above hold it in a type that
//! cannot be copied. [`Socket`] is that type.
//!
//! It is the only thing in this workspace whose destructor closes a socket. Nothing constructs one
//! from a raw descriptor outside this crate, and nothing hands the raw descriptor out beyond it,
//! so a second closer cannot be formed.

use core::{
    marker::PhantomData,
    net::SocketAddr,
};

use nx_service_bsd::{
    BsdSockFd,
    CommandError,
    FcntlOp,
    RecvFlags,
    SendFlags,
    StatusFlags,
};

use crate::{
    addr,
    addr::DecodeAddrError,
    session,
};

/// An open socket, closed when this value is dropped.
///
/// Neither `Copy` nor `Clone`: duplicating one would duplicate the close, and the second close
/// would land on whatever number the service had since reissued.
#[derive(Debug)]
pub struct Socket {
    fd: BsdSockFd,
    /// Keeps the type from being constructed by a struct literal outside this module, so
    /// [`Socket::from_raw_unchecked`] stays the only way to take on the close obligation.
    _not_constructible: PhantomData<()>,
}

impl Socket {
    /// Adopts a descriptor the service just issued, as its sole owner.
    ///
    /// The caller must ensure `fd` names a live socket that nothing else will close, since this
    /// value closes it on drop. A second owner sends its close against a number the service may
    /// have reissued, tearing down an unrelated socket rather than faulting, which is why this is
    /// a safe function.
    pub(crate) const fn from_raw_unchecked(fd: BsdSockFd) -> Self {
        Self {
            fd,
            _not_constructible: PhantomData,
        }
    }

    /// Names the socket, for a command addressed to it.
    ///
    /// Closes nothing: the returned value is the service's number, and this type remains the only
    /// closer. Crate-private, because outside this crate the number has no meaning and handing it
    /// out would be the first half of forming a second owner.
    pub(crate) const fn service_fd(&self) -> BsdSockFd {
        self.fd
    }

    /// Gives up the close obligation, returning the bare descriptor.
    ///
    /// The caller takes on closing it. This exists for the descriptor the service hands back from
    /// a command that has already adopted it elsewhere; ordinary code drops the [`Socket`].
    pub(crate) fn into_service_fd(self) -> BsdSockFd {
        let this = core::mem::ManuallyDrop::new(self);
        this.fd
    }

    /// Opens a socket that can carry the given address.
    ///
    /// The address selects the family, exactly as `std`'s platform layer does it: a caller that
    /// knows where it is about to bind or connect does not also have to name the family, and the
    /// two cannot disagree.
    ///
    /// # Errors
    ///
    /// [`Error::NotConnected`] when the driver is not initialized, and [`Error::Service`] when the
    /// service refused to create the socket.
    pub fn open(addr: &SocketAddr, ty: SockType) -> Result<Self, Error> {
        let domain = match addr {
            SocketAddr::V4(_) => Domain::Ipv4,
            SocketAddr::V6(_) => Domain::Ipv6,
        };
        Self::open_raw(domain, ty)
    }

    /// Opens a socket for a family named directly.
    ///
    /// [`Self::open`] is the form to prefer; this is for the caller that has no address yet.
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_raw(domain: Domain, ty: SockType) -> Result<Self, Error> {
        // The protocol is left for the service to choose. Each of the two types this crate names
        // has exactly one protocol under these families, so naming it would only add a way to
        // disagree with the type.
        const DEFAULT_PROTOCOL: i32 = 0;

        let created = command(|svc| svc.socket(domain as i32, ty as i32, DEFAULT_PROTOCOL))?;

        // SAFETY: the command just issued this descriptor and nothing else has taken it on.
        Ok(Self::from_raw_unchecked(created))
    }

    /// Assigns a local address to the socket.
    ///
    /// # Errors
    ///
    /// [`Error::NotConnected`] when the driver is not initialized, and [`Error::Service`] when the
    /// service refused, which for a bind means the address is already in use or is not one this
    /// console holds.
    pub fn bind(&self, addr: &SocketAddr) -> Result<(), Error> {
        let raw = addr::encode(*addr);
        command(|svc| svc.bind(self.fd, &raw))
    }

    /// Initiates a connection to a peer.
    ///
    /// # Errors
    ///
    /// As [`Self::bind`]; the service reports a refused or unreachable peer through
    /// [`Error::Service`].
    pub fn connect(&self, addr: &SocketAddr) -> Result<(), Error> {
        let raw = addr::encode(*addr);
        command(|svc| svc.connect(self.fd, &raw))
    }

    /// Marks the socket as accepting connections.
    ///
    /// # Errors
    ///
    /// As [`Self::bind`].
    pub fn listen(&self, backlog: i32) -> Result<(), Error> {
        command(|svc| svc.listen(self.fd, backlog))
    }

    /// Takes the next connection off the queue.
    ///
    /// # Errors
    ///
    /// [`Error::Service`] carries the condition a non-blocking socket with nothing queued reports,
    /// which is how a caller polling for a connection tells "none yet" from a real failure.
    /// [`Error::InvalidAddress`] when the connection was accepted but the peer it names cannot be
    /// decoded; the accepted socket is closed before returning, since nothing else can reach it.
    pub fn accept(&self) -> Result<(Self, SocketAddr), Error> {
        let (accepted, peer) = command(|svc| svc.accept(self.fd))?;

        // The service has issued the descriptor, so something must own it from here on. Adopting it
        // before the address is decoded is what closes it on the failing path rather than leaking
        // it.
        // SAFETY: `accept` just issued this descriptor and nothing else has taken it on.
        let socket = Self::from_raw_unchecked(accepted);

        let peer = addr::decode(&peer).map_err(Error::InvalidAddress)?;
        Ok((socket, peer))
    }

    /// Receives into `buf`, reporting how much arrived.
    ///
    /// # Errors
    ///
    /// As [`Self::accept`] for the service's conditions; a non-blocking socket with nothing to read
    /// reports one of them rather than blocking.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Error> {
        command(|svc| svc.recv(self.fd, buf, RecvFlags::empty()))
    }

    /// Receives into `buf`, reporting how much arrived and who sent it.
    ///
    /// # Errors
    ///
    /// As [`Self::accept`], including [`Error::InvalidAddress`] when the datagram arrived but its
    /// sender cannot be named. The bytes are in `buf` either way; only the address is lost.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let (received, peer) = command(|svc| svc.recv_from(self.fd, buf, RecvFlags::empty()))?;
        let peer = addr::decode(&peer).map_err(Error::InvalidAddress)?;
        Ok((received, peer))
    }

    /// Sends `buf`, reporting how much the service took.
    ///
    /// A short count is an ordinary outcome: the caller sends the rest.
    ///
    /// # Errors
    ///
    /// As [`Self::recv`].
    pub fn send(&self, buf: &[u8]) -> Result<usize, Error> {
        command(|svc| svc.send(self.fd, buf, SendFlags::empty()))
    }

    /// Sends `buf` to a named peer, reporting how much the service took.
    ///
    /// # Errors
    ///
    /// As [`Self::recv`].
    pub fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> Result<usize, Error> {
        let raw = addr::encode(*addr);
        command(|svc| svc.send_to(self.fd, buf, SendFlags::empty(), &raw))
    }

    /// Sets whether operations return rather than waiting.
    ///
    /// Reads the current flags before writing them back, so the one flag this changes is the only
    /// one it changes.
    ///
    /// # Errors
    ///
    /// As [`Self::bind`].
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Error> {
        let flags = command(|svc| svc.fcntl(self.fd, FcntlOp::GetFlags))?;

        let mut updated = flags;
        updated.set(StatusFlags::NONBLOCK, nonblocking);

        command(|svc| svc.fcntl(self.fd, FcntlOp::SetFlags(updated)))?;
        Ok(())
    }

    /// Sets whether the local address may be reused while a previous connection lingers.
    ///
    /// What a listener bound to a fixed port needs when its program is relaunched: without it the
    /// port stays unusable for as long as the kernel holds the old connection.
    ///
    /// # Errors
    ///
    /// As [`Self::bind`].
    pub fn set_reuse_address(&self, reuse: bool) -> Result<(), Error> {
        /// Socket-level options, as the BSD headers number them.
        const SOL_SOCKET: i32 = 0xffff;
        /// `SO_REUSEADDR`.
        const SO_REUSEADDR: i32 = 0x0004;

        let enabled: i32 = reuse.into();
        command(|svc| svc.set_sock_opt(self.fd, SOL_SOCKET, SO_REUSEADDR, &enabled))
    }

    /// Closes the socket, reporting what the service said.
    ///
    /// [`Drop`] does the same thing and discards the answer, which is the right behaviour for a
    /// socket going out of scope. This is for the caller that asked to close and is owed a verdict.
    ///
    /// # Errors
    ///
    /// Returns [`CloseFailed::NotConnected`] when the driver is not initialized, and
    /// [`CloseFailed::Service`] when the service refused, which for a close means it did not
    /// recognise the descriptor. The socket is gone from this process either way.
    pub fn close(self) -> Result<(), CloseFailed> {
        let fd = self.into_service_fd();
        session::with_service(|svc| svc.close_fd(fd))
            .map_err(|_| CloseFailed::NotConnected)?
            .map_err(CloseFailed::Service)
    }
}

/// Errors returned by the operations on [`Socket`].
///
/// One type across the surface rather than one per operation, which is what `std`'s platform layer
/// does with `io::Error` and what this layer is for: every command reaches the service the same
/// way, so each fails the same two ways, and the third variant is reachable from exactly the two
/// operations that decode an address.
///
/// [`Socket::close`] keeps its own [`CloseFailed`], because it consumes the socket and so is the
/// one operation whose failure a caller cannot retry.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The socket driver is not initialized
    ///
    /// Occurs when a command is issued before [`crate::initialize`] or after [`crate::exit`].
    /// Nothing was sent, so no socket state changed.
    #[error("The socket service is not connected")]
    NotConnected,

    /// The service refused the command
    ///
    /// Carries the named condition the service reported, which is where the distinction between a
    /// socket that would block, one that was reset, and one that was never bound lives.
    #[error("The socket service refused the command")]
    Service(#[source] CommandError),

    /// The address the service reported cannot be named
    ///
    /// Occurs when a command succeeds but the address it reports back is empty, truncated, or in a
    /// family this layer does not decode. The command itself did what it was asked; only the
    /// address is unusable.
    #[error("The service reported an address that cannot be decoded")]
    InvalidAddress(#[source] DecodeAddrError),
}

/// Errors returned by [`Socket::close`].
#[derive(Debug, thiserror::Error)]
pub enum CloseFailed {
    /// The socket driver is not initialized
    ///
    /// Nothing was sent. The service tore every socket down when the driver exited, so there is
    /// nothing left to close.
    #[error("The socket service is not connected")]
    NotConnected,

    /// The service refused the close
    #[error("The service failed to close the socket")]
    Service(#[source] CommandError),
}

/// The address family a socket carries.
///
/// Each variant's discriminant is the family's own number, so `domain as i32` is what the command
/// takes. The service descends from a BSD stack, which numbers IPv6 28 rather than the 10 a Linux
/// table would give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Domain {
    /// IPv4 (`AF_INET`).
    Ipv4 = 2,
    /// IPv6 (`AF_INET6`).
    Ipv6 = 28,
}

/// What a socket carries, and with what guarantees.
///
/// Each variant's discriminant is the type's own number, so `ty as i32` is what the command takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SockType {
    /// A reliable byte stream (`SOCK_STREAM`, TCP).
    Stream = 1,
    /// Connectionless datagrams (`SOCK_DGRAM`, UDP).
    Dgram = 2,
}

/// Runs one command against the driver's session, flattening the two ways it can fail.
///
/// Every operation above reaches the service the same way and fails the same two ways, so each one
/// would otherwise repeat this pair of `map_err` calls verbatim.
fn command<T>(
    op: impl FnOnce(&nx_service_bsd::BsdService) -> Result<T, CommandError>,
) -> Result<T, Error> {
    session::with_service(op)
        .map_err(|_| Error::NotConnected)?
        .map_err(Error::Service)
}

impl Drop for Socket {
    fn drop(&mut self) {
        // The close is best-effort: the only failure the service reports is a descriptor it does
        // not recognise, which means it is already gone, and a driver that has since been torn
        // down closed every socket with it. Neither leaves anything for a caller to do.
        let _ = session::with_service(|svc| svc.close_fd(self.fd));
    }
}
