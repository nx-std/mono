//! What the C boundary needs of the service and of a context, and the Rust API has no use for.
//!
//! Both blocks are inherent impls on types declared in the crate root, placed here so everything
//! the `ffi` feature adds is reachable from one module rather than scattered through the crate as
//! gated methods.

use nx_sf::{
    cmif::ObjectId,
    service::{
        DomainObject,
        DomainObjectRef,
        DomainRef,
    },
};

use crate::{
    ConnectionKind,
    CreateConnectionError,
    SslContext,
    SslService,
    cmif,
};

/// The operations a C caller's service struct needs, which no Rust caller does.
///
/// Creating a connection under a C-held context cannot sit on
/// [`ForeignSslContext`](super::ForeignSslContext) with the rest of that object's commands,
/// because adopting the object the reply carries is the domain owner's job, and a view onto
/// somebody else's context is not the owner.
impl SslService {
    /// Creates a connection under a context this service created, whose close a C caller took on.
    ///
    /// [`SslContext::create_connection`] is this same command for a context this crate still
    /// holds. The returned [`DomainObject`] is the sole closer for the new connection, which is
    /// what the caller hands on when it writes the service struct the C side will close.
    pub fn create_connection_under(
        &self,
        context: ObjectId,
        kind: ConnectionKind,
    ) -> Result<DomainObject<'_>, CreateConnectionError> {
        let guard = self.pool.acquire();
        // SAFETY: the id came out of a service struct this crate wrote, for an object it created
        // in this domain. All pool domains share one server-side object table (see
        // `connect_cmif`), so the slot this acquired addresses the same object the creating slot
        // did. A stale id, from a context the C side already closed, is answered with an error by
        // the command rather than reaching another object, because the server does not reuse an id
        // while the domain lives.
        let context = DomainObjectRef::from_raw_unchecked(guard.domain(), context.to_raw())
            .ok_or(CreateConnectionError::MissingObject)?;

        let raw_object_id = cmif::create_connection(context, kind)?;
        // SAFETY: `raw_object_id` was just returned by the command on this same domain, and no
        // other `DomainObject` references it.
        DomainObject::from_raw_unchecked(guard.domain(), raw_object_id)
            .ok_or(CreateConnectionError::MissingObject)
    }

    /// The domain root, for a C caller sending commands this crate does not carry.
    ///
    /// The C API exposes the service session so a program can issue its own requests against it.
    /// What comes back borrows: the pool owns every session and closes them all when the service
    /// drops, so a caller that closes this one takes a session out from under the pool.
    pub fn root(&self) -> DomainRef<'_> {
        self.pool.root()
    }
}

impl<'svc> SslContext<'svc> {
    /// Gives up the context, returning the object that owes its close.
    ///
    /// The release half of this type: what comes back is the sole closer for the context, and
    /// dropping it sends the close this type's own drop would have. It exists for the C boundary,
    /// where the obligation is handed to a caller that will close the object itself through the
    /// service struct it is given.
    pub fn into_object(self) -> DomainObject<'svc> {
        self.object
    }
}
