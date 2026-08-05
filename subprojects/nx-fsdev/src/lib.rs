//! # nx-fsdev
//!
//! The filesystem device: what makes `sdmc:/save.dat` reach the SD card.
//!
//! This crate replaces libnx's `runtime/devices/fs_dev.c`, the layer that sits between the C
//! standard library and the `fsp-srv` service. It implements the device traits [`nx_sys_fd`]
//! defines, and it issues the commands [`nx_service_fs`] wraps. It owns neither side: the
//! descriptor table below it and the IPC client above it are both somebody else's crate.
//!
//! ## Why this is a crate of its own
//!
//! A `printf` and an `fopen` travel the same road for most of their length. Newlib calls
//! `libsysbase`, `libsysbase` looks the descriptor up in [`nx_sys_fd`]'s table, and the table
//! dispatches to whichever device backs it. Up to that point nothing knows what a filesystem is.
//!
//! What the table dispatches *into* is the part that does: something has to turn "open
//! `/nx-tests-fs/a.txt` for writing" into an `IFileSystem` command with a 0x301-byte path buffer,
//! and to hold the session that command is addressed to. That is this crate, and it is separate
//! from both neighbours because neither should grow a dependency on the other: the descriptor
//! table must stay usable by a console or a socket layer that has never heard of `fsp-srv`, and
//! the `fsp-srv` client must stay usable by a caller who wants commands rather than descriptors.
//!
//! ## What is mounted, and where the session lives
//!
//! A mount pairs a name with an `IFileSystem` the server opened: `sdmc` with the SD card,
//! `save` with a save-data filesystem. [`mount`] holds those pairs, and each one is registered
//! with [`nx_sys_fd`]'s registry so that a path naming it resolves here.
//!
//! The session every one of those objects belongs to is a different lifetime, and it is not this
//! crate's to create. `fsp-srv` is reached through the Service Manager, which the runtime
//! bootstraps long before any filesystem is mounted, so the runtime connects and hands the service
//! down to [`service::set`]. What this crate owns is the *storage*: one process-wide slot the
//! device operations borrow from, because they must, and the runtime writes to once.
//!
//! ## Objects are named, not held
//!
//! An open filesystem, file or directory is a server-side object addressed by an id inside the
//! session's domain. The natural Rust wrapper for one borrows the service it belongs to, and a
//! device registered for the life of the process cannot hold a borrow of something behind a lock.
//!
//! So nothing here holds a wrapper. A device records the *id* of its filesystem, a file records
//! the id of its file, and every operation rebuilds the wrapper for the length of one command and
//! hands the close obligation straight back. Only an explicit close, a descriptor closing or a
//! device unmounting, lets the wrapper drop and take the object with it. This is the same
//! arrangement the `fs*` C surface already uses, for the same reason.
//!
//! ## A device outlives its mount
//!
//! [`nx_sys_fd`]'s registry holds `&'static dyn Device`, so a device has to be reachable forever
//! once it is registered. Unmounting therefore cannot free one. Rather than leak a fresh device on
//! every mount, a device is created once per *name* and reused: mounting fills its state,
//! unmounting empties it, and mounting the same name again refills the one already there. What
//! leaks is bounded by how many distinct names a process ever mounts, and an unmounted device
//! holds no session, no path and no memory beyond its own name.
//!
//! ## Where the path is put together
//!
//! [`nx_sys_fd`] strips the `"name:"` prefix before a device sees a path, so what arrives is the
//! device's own business, and it may be relative. Joining it onto the working directory, and
//! rejecting what will not fit the fixed buffer `fsp-srv` takes, happens in [`path`]: once, at
//! the top of every operation that takes a path, before any command is built.
//!
//! Note that the working directory is per device, not per process. That is libnx's arrangement and
//! the C standard library's: `chdir("sdmc:/a")` moves the SD card's directory and leaves every
//! other mount where it was.
//!
//! ## What is not implemented
//!
//! The save-data mounts are aliased to panicking stubs. Each one has to build a save-data
//! attribute the server matches on, and none of that is exercised by anything in this workspace
//! yet. They panic rather than fall back to libnx because a fallback would run against a `g_fsSrv`
//! this workspace zeroed, which does not fail: it parks forever inside libnx's session manager.
//! A panic names the missing command; a hang names nothing.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The mount table, the working directory of each device, and the entry cache of an open directory
// walk are all heap-backed.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

// The device layer exists to be reached from C: the descriptor table dispatches into it through
// the `fsdev*` entry points, and nothing in Rust names it yet. It is gated with the surface it
// serves so a build without that surface carries none of it.
#[cfg(feature = "ffi")]
pub(crate) mod device;
#[cfg(feature = "ffi")]
pub(crate) mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
#[cfg(feature = "ffi")]
pub(crate) mod mount;
#[cfg(feature = "ffi")]
pub(crate) mod path;
pub mod service;
