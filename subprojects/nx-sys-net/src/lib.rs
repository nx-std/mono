//! # nx-sys-net
//!
//! The socket driver: what makes `socket()`, `bind()` and `recv()` reach the network.
//!
//! This crate backs `std::sys::net`, the platform layer `std::net`'s `TcpStream`, `TcpListener`
//! and `UdpSocket` are built on. It implements the device traits [`nx_sys_fd`] defines, and it
//! issues the commands [`nx_service_bsd`] wraps. It owns neither side: the descriptor table below
//! it and the IPC client above it are both somebody else's crate.
//!
//! ## Why this is a crate of its own
//!
//! [`nx_service_bsd`] speaks to the `bsd:u`/`bsd:s` service and stops there. Its descriptors are
//! the service's own numbers, its addresses are undifferentiated bytes, and its failures are named
//! conditions in the service's Linux numbering. None of that is what a C caller holds: a C caller
//! has a descriptor from the process-wide table, a `struct sockaddr`, and an `errno` slot in
//! newlib's numbering. Something has to sit between them, hold the session both sides assume, and
//! translate in each direction. That is this crate.
//!
//! It is separate from both neighbours because neither should grow a dependency on the other. The
//! descriptor table must stay usable by a console or a filesystem that has never heard of the BSD
//! service, and the BSD client must stay usable by a caller who wants commands rather than
//! descriptors.
//!
//! ## Two descriptor spaces, and the map between them
//!
//! There are two kinds of number here and conflating them is the trap:
//!
//! - The **service's** descriptor, [`nx_service_bsd::SocketFd`]. Issued by the BSD service,
//!   meaningful only in a command sent to it.
//! - The **process's** descriptor, [`nx_sys_fd::table::Fd`]. Issued by the descriptor table, and
//!   what every C caller passes and receives.
//!
//! They are never equal and neither can be derived from the other. The map between them is one
//! direction only: a process descriptor owns a [`device::SocketFile`], and that file holds the
//! service descriptor. Recovering it is what [`nx_sys_fd::table::with_file`] is for, and
//! [`device::sock_of`] is the one place in this crate that does it.
//!
//! This arrangement is why `read()`, `write()` and `close()` need nothing from this crate's C
//! surface: they are ordinary descriptor operations, so the table dispatches them into
//! [`device::SocketFile`] like any other file. Only the calls that are *not* descriptor
//! operations - `send`, `bind`, `listen`, and the rest of the BSD surface - go through the map
//! explicitly.
//!
//! ## Where the session lives
//!
//! Every command needs a connected [`nx_service_bsd::BsdService`], and no C caller passes one. So
//! the crate holds one process-wide, established by [`driver::initialize`] and released by
//! [`driver::exit`], the lifecycle the C `socketInitialize`/`socketExit` pair exposes. Unlike the
//! resolver's session, it is not connected lazily on first use: the C contract is that a program
//! initializes the socket driver before calling anything, and a lazy connect would paper over the
//! omission with a config nobody chose.
//!
//! ## "Process-wide" when the process links this crate twice
//!
//! The session above is a `static`, and a `static` is only process-wide if the crate holding it is
//! linked once. That is the ordinary case: every Rust crate in this workspace reaches a program
//! through the single umbrella static library, so there is one compilation and one slot.
//!
//! It stops being the ordinary case as soon as a program links a static library of its own. An
//! application library that wants sockets -- a netloader, a telemetry client -- takes this crate as
//! a dependency and is compiled into its own archive. Cargo resolves features per build, and two
//! builds that disagree on a feature produce two crate hashes; a `static` is mangled with that hash,
//! so the program ends up with two slots. The half that ran `socketInitialize` fills one, and the
//! half that calls [`Socket::open`] reads the other and finds nothing.
//!
//! **That failure is silent.** Nothing is reported at link time, because the two symbols are
//! genuinely different symbols. It surfaces at run time as [`Error::NotConnected`] from a driver the
//! program did initialize, which reads as a bug anywhere except where it is.
//!
//! ### The `extern-state` feature
//!
//! So the slot has a spelled-out symbol rather than a mangled one, and the crate can be built to
//! borrow it rather than define it:
//!
//! - **Default:** the session is defined here. The static library that owns the driver -- the one
//!   whose `socketInitialize` runs -- is built this way.
//! - **`extern-state`:** the session is *declared* and resolved by the linker to the definition in
//!   the other archive. Every other static library in the program is built this way.
//!
//! **Exactly one static library in a program may leave `extern-state` off.** Two definitions is a
//! duplicate-symbol error, which is loud and easily fixed; zero is an undefined-symbol error, which
//! is equally loud. Both are better than the silent version this replaces, and that is the point of
//! spelling the symbol out: it converts a runtime mystery into a link-time complaint.
//!
//! ### What a consumer must not do
//!
//! A build with `extern-state` must not also enable `ffi`. The C surface is `#[no_mangle]`
//! throughout, so a second archive carrying it collides on every entry point. The division is the
//! same one the session follows: one archive owns the driver and exposes it to C, and the others
//! borrow it from Rust.
//!
//! `extern-state` must never change the layout of anything it touches. The two builds agree on
//! [`nx_service_bsd::BsdService`]'s layout because they are the same source at the same version, and
//! that is the whole of the guarantee; a feature that altered a field would break it silently.
//!
//! ## The service's error numbering is not the caller's
//!
//! The BSD service was built against Linux's error numbering and the C library here uses newlib's.
//! The two agree only below 35, so a code copied from the wire into an `errno` slot reports the
//! wrong failure. [`nx_service_bsd`] therefore hands up a named [`nx_service_bsd::PosixError`] and
//! leaves the number to whoever knows which numbering their caller reads. This crate is that
//! layer, and the table is in [`ffi::errno`], with the C surface, because a number is the only
//! thing a C caller can be told, and nothing above the C boundary needs one.
//!
//! ## What the version field is, and why it defaults low
//!
//! The connect handshake declares which revision of the service interface the client speaks. The
//! revision tracks firmware, and the service answers any revision up to its own, so declaring less
//! than the firmware supports is safe on every firmware while declaring more is not.
//!
//! The firmware version is a fact the runtime holds, and nothing below the runtime may reach for
//! it, so this crate cannot resolve the revision itself. [`nx_service_bsd::ConnectOptions`]
//! therefore carries it as a parameter and defaults to [`nx_service_bsd::ConfigVersion::V1`],
//! which every firmware accepts. A caller that knows its firmware raises it with
//! [`nx_service_bsd::ConfigVersion::for_firmware`].
//!
//! This is the one place the crate knowingly differs from the C socket driver it replaces, which
//! derives the revision from the firmware it queried at startup.
//!
//! ## no-std
//!
//! The crate is `#![no_std]` and uses `alloc` for the session pool below it and for the message
//! vectors the scatter-gather calls assemble; the umbrella `nx-std` crate owns the single
//! `#[global_allocator]`.
#![no_std]
// Writing the thread-local the C surface reports a transport failure through needs the attribute.
#![cfg_attr(feature = "ffi", feature(thread_local))]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The multi-message calls assemble a contiguous request out of a caller's `iovec` array, and the
// device hands the table a boxed file.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

pub mod addr;
pub mod device;
pub mod driver;
pub mod readiness;
pub mod session;
pub mod socket;

#[cfg(feature = "ffi")]
pub mod ffi;

// What a socket is made of is the service's vocabulary, not this crate's, so the types come from
// the client rather than being restated here. Re-exported so a caller opening a socket needs only
// this crate.
pub use nx_service_bsd::{
    Domain,
    Protocol,
    SockType,
};

pub use self::{
    driver::{
        exit,
        initialize,
    },
    readiness::{
        Event,
        Events,
        Interest,
        Readiness,
        Selector,
        Token,
    },
    socket::{
        Error,
        Socket,
    },
};
