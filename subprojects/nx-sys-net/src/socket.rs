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

use core::marker::PhantomData;

use nx_service_bsd::BsdSockFd;

use crate::session;

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
    Service(#[source] nx_service_bsd::CommandError),
}

impl Drop for Socket {
    fn drop(&mut self) {
        // The close is best-effort: the only failure the service reports is a descriptor it does
        // not recognise, which means it is already gone, and a driver that has since been torn
        // down closed every socket with it. Neither leaves anything for a caller to do.
        let _ = session::with_service(|svc| svc.close_fd(self.fd));
    }
}
