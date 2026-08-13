//! The host that pushed this program, and the connection the report goes back over.
//!
//! The only reader a run can be driven from: the console's screen keeps nothing and the card has to
//! be fetched by hand, so a run that reports to neither reports to nobody until somebody goes and
//! looks.
//!
//! # The socket driver is the caller's
//!
//! Bringing the driver up needs the service-manager session the runtime owns, and a suite that
//! already brought one up for its own cases would have a second one taken down underneath it. So
//! this connects over a driver that is already running and never starts or stops one; a caller that
//! has not brought one up gets [`ConnectError::Socket`] and files its report to the card alone.

use core::net::{
    Ipv4Addr,
    SocketAddr,
    SocketAddrV4,
};

use nx_sys_net::{
    SockType,
    Socket,
};

/// The port the host listens for a program's output on.
const HOST_PORT: u16 = 28771;

/// A connection to the host, open for as long as there is something to send it.
pub struct Host {
    /// The connected socket, closed when this is dropped.
    sock: Socket,
}

impl Host {
    /// Connects to `addr`, over a socket driver the caller has already brought up.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectError::Socket`] when no socket could be opened, which is what a caller that
    /// brought no driver up sees, and [`ConnectError::Refused`] when the host is reachable but
    /// nothing is listening on it. Neither is a failure of the run: the report still reaches the
    /// card.
    pub fn connect(addr: Ipv4Addr) -> Result<Self, ConnectError> {
        let addr = SocketAddr::V4(SocketAddrV4::new(addr, HOST_PORT));

        let sock = Socket::open(&addr, SockType::Stream).map_err(ConnectError::Socket)?;
        // Blocking, unlike everything else this program does with a socket. The host pushed this
        // program moments ago, so it either accepts at once or refuses at once; there is no third
        // case worth spending a deadline and a retry loop on.
        sock.connect(&addr).map_err(ConnectError::Refused)?;

        Ok(Self { sock })
    }

    /// Sends `text`, in as many writes as the socket takes it in.
    ///
    /// A send that stops part-way loses the rest of the document and nothing else, so it ends the
    /// write rather than being reported: the card has the same document, and a report that failed
    /// to reach the host is not a run that failed.
    pub fn write(&self, text: &str) {
        let bytes = text.as_bytes();
        let mut sent = 0usize;

        while sent < bytes.len() {
            match self.sock.send(&bytes[sent..]) {
                Ok(0) => return,
                Ok(taken) => sent += taken,
                Err(_) => return,
            }
        }
    }

    /// Closes the connection.
    ///
    /// The failure is dropped: the host has either received the document by now or it has not, and
    /// there is nothing left for this program to do about either.
    pub fn close(self) {
        let _ = self.sock.close();
    }
}

/// Errors returned by [`Host::connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// No socket could be opened
    ///
    /// Occurs when the socket driver is not running, which is what a caller that brought none up
    /// sees.
    #[error("a socket could not be opened")]
    Socket(#[source] nx_sys_net::Error),

    /// The host did not accept the connection
    ///
    /// Occurs when nothing is listening on the host, which is the ordinary case for a run that was
    /// pushed without asking for its output back.
    #[error("the host did not accept the connection")]
    Refused(#[source] nx_sys_net::Error),
}
