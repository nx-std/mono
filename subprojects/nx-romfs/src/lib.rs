//! # nx-romfs
//!
//! The read-only filesystem device: what makes `romfs:/config.json` reach the
//! image a program carries with it.
//!
//! This crate replaces libnx's `runtime/devices/romfs_dev.c`. It implements the
//! device traits [`nx_sys_fd`] defines, and it reads its bytes either from a
//! file on another mounted device or from a storage object `fsp-srv` opened. It
//! owns neither end: the descriptor table below it and the image's byte source
//! above it are both somebody else's.
//!
//! ## What a romfs image is
//!
//! A header, four tables, and a blob of file data, all at fixed offsets from
//! the start of the image. Two of the tables hold the directory and file
//! entries; the other two are hash buckets that chain into them. An entry names
//! its parent, its next sibling, and its first child, so the whole tree is
//! reachable by following 32-bit offsets, and a lookup by name is a hash of
//! (parent offset, name) followed by a walk down one chain.
//!
//! Nothing in the image is written, ever. That is the property the whole design
//! leans on: the four tables are read once at mount time and never re-read, so
//! every path lookup afterwards is pure computation over memory this crate
//! owns, and only reading a file's *contents* costs a command.
//!
//! ## The tables are copied, not streamed
//!
//! Mounting reads all four tables into the heap. A directory of a few hundred
//! entries is a few kilobytes, and paying for it once buys every later lookup
//! at no I/O at all: `open` on a mounted image issues exactly one read, for the
//! file data, and `stat` issues none. Streaming the tables instead would turn
//! each path component into a round trip.
//!
//! The tables are also untrusted. They come off an SD card, so every offset
//! read out of one is bounds-checked against the table it indexes before it is
//! followed ([`image`]), and a violation is reported rather than trusted.
//!
//! ## Where the bytes come from
//!
//! Two sources, and the distinction is not cosmetic ([`source`]):
//!
//! - **A file on another device.** A homebrew `NRO` carries its image appended
//!   to itself, so mounting it means reading the very file the program was
//!   launched from, through whichever device that file lives on. The source is
//!   a [`nx_sys_fd::device::File`], which is what any mounted device produces:
//!   this crate never learns whether that is an SD card.
//! - **A storage object.** A packaged program's data partition is an `IStorage`
//!   the `fsp-srv` session opened, and reading it is a command on that object.
//!
//! The session those objects belong to is not this crate's to create. It is
//! bootstrapped by the runtime and lives in [`nx_fsdev::service`], where the
//! filesystem device and this one both reach it.
//!
//! ## Which image is *mine* is not a question asked here
//!
//! libnx answers it with `romfsMountSelf`, which branches on whether the
//! process is an `NSO` or an `NRO` and picks a source accordingly. That branch
//! cannot live below the runtime: the output kind is resolved by which entry
//! crate a binary links, not by asking at run time, and nothing under
//! `nx-rt-*` may ask.
//!
//! So this crate offers one entry point per source and no way to pose the
//! question at all: [`mount::from_file`] and [`mount::from_current_process`]
//! are different functions, not one function with a flag. The entry crates each
//! call the one that is right for them, unconditionally.
//!
//! ## A device outlives its mount
//!
//! [`nx_sys_fd`]'s registry holds `&'static dyn Device`, so a device has to be
//! reachable forever once registered. Unmounting therefore empties a device
//! rather than freeing it, and mounting the same name again refills the one
//! already there. What leaks is bounded by how many distinct names a process
//! ever mounts, and an unmounted device holds no image, no source and no memory
//! beyond its own name. This is [`nx_fsdev`]'s arrangement, for the same
//! reason.
//!
//! ## No timestamps
//!
//! A romfs image records none. libnx reports the wall clock at the moment of
//! mounting for every entry, which is not when anything was written and is the
//! same value for a file and the directory holding it. [`nx_sys_fd`] lets a
//! device say it keeps no timestamps, so this one says that.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The four tables of a mounted image, the mount table, and the devices in it are all heap-backed.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

// Only the C surface is gated. Unlike the filesystem device, whose every caller arrives through
// `fsdev*`, this crate has a Rust consumer that is not the C boundary: the entry crate that mounts
// a program's own image calls `mount` directly, and it must be able to do so in a build that emits
// no override symbols at all.
#[cfg(feature = "ffi")]
pub mod ffi;

pub(crate) mod device;
pub(crate) mod image;
pub mod mount;
pub(crate) mod source;
