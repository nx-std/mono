//! The C boundary's view of the objects this crate speaks to.
//!
//! This module defines no `__nx_*` symbol of its own. What it holds is the shape another crate's
//! C entry points address: a context or a connection whose close obligation a C caller took on,
//! which this crate can send commands to without owning. That is built for a C boundary, so a
//! pure-Rust link should not pay for it, and the `ffi` feature is what keeps it out of one.
//!
//! # The two directions
//!
//! A C caller reaches an object here in one of two ways, and both are covered:
//!
//! - It **already holds one**, having been given the service struct earlier. The types below read
//!   that struct and address what it names, closing nothing.
//! - It is **about to hold one**, because the C API it called creates an object. That is the
//!   service's job rather than a view's, since only the owner of the domain can adopt the object a
//!   reply carries; see [`SslService::create_connection_under`](crate::SslService::create_connection_under).

mod connection;
mod context;
mod service;

pub use self::{
    connection::ForeignSslConnection,
    context::ForeignSslContext,
};
