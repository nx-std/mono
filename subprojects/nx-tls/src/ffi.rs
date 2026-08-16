//! The `ssl` C surface.
//!
//! Everything the `ffi` feature adds lives under here, including the state the surface needs. The
//! process-wide connection in [`session`] and the firmware questions in [`firmware`] are not
//! themselves C entry points, but nothing else reaches them: they exist because a C caller passes
//! no service and names no firmware, and a crate linked for Rust alone should compile neither.
//!
//! # Layout
//!
//! One module per interface the surface addresses, which is how upstream's `ssl.h` is grouped:
//!
//! - [`service`] brings the service up and down, and answers the commands the service itself takes
//! - [`context`] answers what an `ISslContext` takes
//! - [`connection`] answers what an `ISslConnection` takes
//! - [`socket`] holds the three descriptor hand-offs, which upstream keeps in its socket driver
//!   rather than its `ssl` client, and which answer in `errno` rather than a result code
//!
//! Three more hold what they all share, one per thing that crosses: [`object`] reaches what a
//! caller's service struct names, [`buffer`] borrows the memory a caller lent, and [`result`] says
//! what a command answers with.

mod buffer;
mod connection;
mod context;
mod firmware;
mod object;
mod result;
mod service;
mod session;
mod socket;
