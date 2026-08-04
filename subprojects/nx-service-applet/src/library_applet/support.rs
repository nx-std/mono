//! Helpers shared by the library applet interfaces.

use nx_sf::service::{
    DomainObject,
    DomainRef,
};

/// Re-anchors a server-emitted object id to the domain that produced it.
///
/// The object a reply hands back borrows the dispatch that carried it, and that
/// borrow ends with the call; the wrappers built from it outlive the call by
/// design. Releasing the close obligation and re-adopting it against `domain`
/// moves that one obligation onto the longer lifetime instead of duplicating it,
/// which is exactly the adoption/release pair, not a second owner.
///
/// # Panics
///
/// Panics if `raw_object_id` is zero. A server that answered success owes a live
/// object id, so a zero here is a protocol violation rather than a value any
/// caller can act on.
pub(super) fn reanchor_object(domain: DomainRef<'_>, raw_object_id: u32) -> DomainObject<'_> {
    // SAFETY: The id was released from the reply's own owner, so this adopts a
    // transferred obligation rather than minting a second one, and the server
    // emitted it for an object inside `domain`.
    DomainObject::from_raw_unchecked(domain, raw_object_id)
        .expect("server-emitted object id is non-zero")
}
