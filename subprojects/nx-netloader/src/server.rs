//! The two sockets a host reaches this console through, and the discovery it answers on one of
//! them.
//!
//! Both are bound to the same port and neither blocks: the runner that owns them is also drawing a
//! screen and watching for a button, so every look at the network has to return whether or not
//! anything was there.

use alloc::string::{
    String,
    ToString as _,
};
use core::net::{
    Ipv4Addr,
    SocketAddr,
    SocketAddrV4,
};

use nx_sys_net::{
    Error as SocketError,
    SockType,
    Socket,
};

use crate::{
    CLIENT_PORT,
    SERVER_PORT,
    transfer,
    transfer::Outcome,
};

/// What a host broadcasts to find a console.
const PING: &[u8] = b"nxboot";

/// What the console answers, which is how the host learns its address.
const PONG: &[u8] = b"bootnx";

/// The largest datagram a discovery ping can arrive in.
///
/// The ping is six bytes; anything longer belongs to somebody else and only has to be recognised as
/// not being a ping.
const PING_BUFFER: usize = 16;

/// The listening sockets, once they are bound.
pub struct Server {
    /// Answers discovery pings so the host can find this console.
    discovery: Socket,
    /// Accepts the transfer connection.
    listener: Socket,
    /// Where a received program is written, which the caller chose.
    drop_dir: String,
}

impl Server {
    /// Binds both sockets and starts listening.
    ///
    /// # Errors
    ///
    /// [`OpenServerError`] naming the socket that could not be prepared. Nothing is left open: the
    /// sockets opened before the failure are closed on the way out.
    pub fn open(drop_dir: &str) -> Result<Self, OpenServerError> {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, SERVER_PORT));

        let discovery = Socket::open(&addr, SockType::Dgram).map_err(OpenServerError::Discovery)?;
        discovery.bind(&addr).map_err(OpenServerError::Discovery)?;
        discovery
            .set_nonblocking(true)
            .map_err(OpenServerError::Discovery)?;

        let listener = Socket::open(&addr, SockType::Stream).map_err(OpenServerError::Listener)?;

        // A run that ended with a connection still in the kernel's hands would otherwise leave the
        // port unusable for as long as that connection lingers, and the runner is relaunched after
        // every program it hands off.
        listener
            .set_reuse_address(true)
            .map_err(OpenServerError::Listener)?;
        listener.bind(&addr).map_err(OpenServerError::Listener)?;
        listener
            .set_nonblocking(true)
            .map_err(OpenServerError::Listener)?;
        listener.listen(1).map_err(OpenServerError::Listener)?;

        Ok(Self {
            discovery,
            listener,
            drop_dir: drop_dir.to_string(),
        })
    }

    /// Closes both sockets and binds them again.
    ///
    /// What a console that has slept needs: its network went away and came back, and the sockets
    /// bound to the old one answer nothing. Binding is what fails while the network is still down,
    /// so a caller retries rather than treating one failure as final.
    ///
    /// # Errors
    ///
    /// As [`Self::open`]. Nothing is left open, so the call can simply be made again later.
    pub fn reopen(self) -> Result<Self, OpenServerError> {
        // The directory outlives the sockets: it is the caller's choice, and rebuilding the sockets
        // is not a change of mind about where programs go.
        let drop_dir = self.drop_dir.clone();
        drop(self);
        Self::open(&drop_dir)
    }

    /// Answers a pending discovery ping, if one has arrived.
    ///
    /// A host that was not told an address finds the console by broadcasting a ping and waiting for
    /// this reply, so this has to be called regularly for as long as the runner is willing to be
    /// found.
    ///
    /// Returns whether a ping was answered. Nothing waiting is the ordinary case on a socket nobody
    /// is pinging, and is reported as `Ok(false)` rather than as a failure.
    ///
    /// # Errors
    ///
    /// [`SocketError`] when the socket has failed in a way waiting will not mend, which means the
    /// server needs rebuilding before any host can find this console again.
    pub fn answer_discovery(&self) -> Result<bool, SocketError> {
        let mut ping = [0u8; PING_BUFFER];

        let (len, host) = match self.discovery.recv_from(&mut ping) {
            Ok(received) => received,
            // Nothing waiting is the ordinary case; the caller is told so and asks again later.
            Err(err) if err.is_would_block() => return Ok(false),
            Err(err) => return Err(err),
        };

        // A datagram that is not the ping is somebody else's; the socket is still good.
        if ping.get(..len) != Some(PING) {
            return Ok(false);
        }

        // The host listens for the answer on a port of its own, not on the one it asked from.
        let reply_to = SocketAddr::new(host.ip(), CLIENT_PORT);

        // A reply that does not arrive leaves the host to ask again, which it does; failing the
        // whole server over one lost datagram would be a worse answer than staying up.
        let _ = self.discovery.send_to(PONG, &reply_to);

        Ok(true)
    }

    /// Receives one program, if a host is connecting.
    ///
    /// Returns `Ok(None)` when no host is connecting, which is the ordinary answer on an idle
    /// console and is why this can be called from the runner's own loop.
    ///
    /// # Errors
    ///
    /// [`SocketError`] when the listening socket has failed rather than merely having nothing
    /// queued. The console taking its network down, which it does whenever it sleeps, invalidates
    /// the socket while leaving the runner running and apparently well, so a caller treats this as
    /// "rebuild the server" rather than "this transfer failed".
    pub fn receive(
        &self,
        extra_arg: Option<&str>,
        progress: &mut dyn FnMut(&str, usize, usize),
    ) -> Result<Option<Outcome>, SocketError> {
        let (connection, peer) = match self.listener.accept() {
            Ok(accepted) => accepted,
            // Nobody waiting is the ordinary case.
            Err(err) if err.is_would_block() => return Ok(None),
            Err(err) => return Err(err),
        };

        // Whether an accepted socket inherits the listening socket's non-blocking mode is left open
        // by the interface, and the reads below bound their own patience, so it is set here rather
        // than assumed either way. A socket that will not take the setting would block the runner
        // forever on its first quiet moment, so this fails the transfer rather than proceeding.
        if let Err(err) = connection.set_nonblocking(true) {
            return Ok(Some(Outcome::Failed {
                reason: alloc::format!("the connection could not be configured: {err}"),
            }));
        }

        Ok(Some(transfer::receive(
            &connection,
            peer,
            &self.drop_dir,
            extra_arg,
            progress,
        )))
    }
}

/// Errors returned by [`Server::open`] and [`Server::reopen`].
#[derive(Debug, thiserror::Error)]
pub enum OpenServerError {
    /// The socket that answers discovery could not be prepared
    ///
    /// Occurs when the datagram socket cannot be opened, bound or made non-blocking. Binding is
    /// what fails while the console has no network, so this is the variant a caller retries.
    #[error("The discovery socket could not be bound")]
    Discovery(#[source] SocketError),

    /// The socket that accepts transfers could not be prepared
    ///
    /// Occurs when the stream socket cannot be opened, configured, bound or listened on. The
    /// discovery socket was already open and is closed before this is returned.
    #[error("The listening socket could not be bound")]
    Listener(#[source] SocketError),
}
