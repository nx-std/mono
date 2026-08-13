//! # nx-netloader
//!
//! The console half of the netloader protocol: how a program gets onto this console over the
//! network without anybody touching it.
//!
//! A host running `nxlink`, or `cargo nx link` which speaks the same protocol, finds a console by
//! broadcasting a ping, opens a connection to it, and sends a program. This crate is what answers.
//!
//! ## The exchange, in order
//!
//! 1. The host broadcasts `nxboot` over UDP to find a console willing to receive; the console
//!    answers `bootnx`, and the host takes the address that reply came from as the console's.
//! 2. The host connects over TCP and sends the file name, then the file's length, and waits for a
//!    status word before sending anything more.
//! 3. The file arrives as a run of length-prefixed compressed chunks; the console inflates them into
//!    the file it reserved, and answers with a second status word once the stream ends.
//! 4. The host sends the command line as a run of NUL-terminated arguments.
//!
//! Every length and status word on the wire is a little-endian 32-bit integer, which is this
//! console's own layout, so they are read and written without byte-swapping.
//!
//! ## The stream ends where the decompressor says it does
//!
//! Nothing on the wire marks the last chunk. The host writes chunks until its compressor is
//! exhausted and then waits for the answer, so the console learns the transfer is over only by the
//! decompressor reporting the stream complete. That rules out any interface that decompresses a
//! whole buffer at once: the chunks have to be fed through a streaming inflate whose status is what
//! ends the loop.
//!
//! The stream is wrapped rather than raw, so the trailing checksum is part of what is verified.
//!
//! ## What the host is, for reference
//!
//! The other half lives outside this workspace, in the `nx-netloader` crate of the tooling
//! repository. The constants here are the same constants: the ports, the chunk ceiling, the command
//! line ceiling and the refusal codes all have to agree, and there is no shared crate to make them
//! agree automatically.
//!
//! ## no-std
//!
//! The crate is `#![no_std]` and uses `alloc` for the inflate state and the buffers a transfer
//! needs. It links as a static library of its own, alongside the one the C runtime already links,
//! which is sound because the panic handler and the global allocator resolve to a single copy
//! shared by both.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The inflate state is boxed, and a transfer buffers what it is about to write.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

pub mod server;
pub mod transfer;

#[cfg(feature = "ffi")]
pub mod ffi;

/// The port the server listens on.
///
/// One number covers both halves of the protocol: a UDP socket that answers the host's discovery
/// ping, and a TCP socket that accepts the transfer itself.
pub const SERVER_PORT: u16 = 28280;

/// The UDP port a discovery reply is sent back to.
///
/// The host listens for the answer on a port of its own rather than on the one it asked from.
pub const CLIENT_PORT: u16 = 28771;

pub use self::server::Server;
