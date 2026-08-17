//! Waiting until sockets are ready.
//!
//! Every other operation on a [`Socket`](crate::Socket) acts on one socket and returns when that
//! socket is done. This one is the exception: it takes a set of them, blocks until any is ready,
//! and reports which. That is what an event loop is built out of, and it is the only operation here
//! that a caller cannot assemble from the others.
//!
//! ## The set is a value, because the service has nowhere to keep one
//!
//! The service offers `Poll` and `Select`, and no registration interface: nothing here corresponds
//! to `epoll` or `kqueue`, where a program declares its interest once and the kernel remembers it.
//! Each wait carries the whole set.
//!
//! So somebody has to hold the set between waits, and [`Selector`] is that somebody. A caller
//! registers each socket once and waits many times, which is the shape an event loop is written
//! against; the copy of the set that goes out with every wait is the service's requirement rather
//! than a choice made here, and it is made where a caller does not have to think about it.
//!
//! `Select` is not wrapped at all. It answers in bitmaps keyed by descriptor number, and the
//! numbers a bitmap would be keyed by are the service's own, which no caller of this crate holds.
//! `Poll` carries one entry per socket, so the correspondence between what was asked and what was
//! answered survives without a caller having to reconstruct it.
//!
//! ## Ending a wait that nothing is going to end on its own
//!
//! A wait ends when a socket becomes ready or the timeout runs out, and an event loop that wants to
//! be told about anything else -- work handed to it, a shutdown, a socket it should start watching
//! -- is asked from a thread that is not the one waiting. [`Waker`] is the second thread's way in:
//! it holds one end of a channel whose other end is inside the selector, and sending on it ends the
//! wait. What that channel is made of, and why, is on [`Waker`] itself.

mod event;
mod interest;
mod selector;
mod wake;

pub use self::{
    event::{
        Event,
        Events,
        Readiness,
        Token,
    },
    interest::Interest,
    selector::{
        DeregisterError,
        RegisterError,
        ReregisterError,
        SelectError,
        Selector,
    },
    wake::{
        OpenWakerError,
        WakeError,
        Waker,
    },
};
