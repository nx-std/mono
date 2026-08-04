//! # nx-sys-fd
//!
//! Virtual file descriptor table and device registry for the C standard library.
//!
//! This crate replaces the two newlib/libgloss translation units that give Horizon homebrew a
//! Unix-shaped I/O surface: `libsysbase/handle_manager.c` (the file descriptor table) and
//! `libsysbase/iosupport.c` (the device registry). Both are vendored under
//! `subprojects/sysroot/libgloss/libsysbase/` and are replaced at link time by
//! `sys_fd_override.ld`.
//!
//! ## Why this crate exists
//!
//! Horizon has no file descriptors. It has kernel handles, sessions and shared memory, and none of
//! those are integers a C program can `write()` to. Every `printf` in a homebrew binary
//! nonetheless ends up calling `_write_r(reent, 1, buf, len)`, because that is what newlib emits.
//! Something has to translate the integer `1` into a device that knows how to render bytes.
//!
//! That translation is what this crate provides. It owns the mapping from descriptor number to
//! open object, and the registry of devices that objects can be backed by. It does not implement
//! any device itself: a console, a filesystem or a socket layer registers with this crate and
//! supplies its own behaviour.
//!
//! ## What a device is asked to implement
//!
//! Three traits, not one, and the split is the crate's central design decision. [`device::Device`]
//! owns a path namespace and is shared by every descriptor opened against it; [`device::File`] is
//! one open file, owned by the descriptor that opened it; [`device::Dir`] is one open directory
//! walk. The reasoning, including why [`device::Device`] still carries `write` and `read` of its
//! own, is in [`device`].
//!
//! ## The two tables, and why they are separate
//!
//! There are two distinct questions, and conflating them is the classic design trap:
//!
//! 1. **How is an open object named?** An integer descriptor indexes the descriptor table, which holds one
//!    slot per descriptor.
//! 2. **How is an object's behaviour dispatched?** Each open object names a device, and the device
//!    holds the operations. Devices live in the device registry, indexed separately.
//!
//! The two indices are unrelated. Descriptor 1 may be backed by device 1; descriptor 7 may also be
//! backed by device 1. Devices are shared, descriptors are not. Keeping the tables separate is
//! what lets several descriptors sit on one device without duplicating its operations, and it is
//! the arrangement the C consumers already expect.
//!
//! ## The descriptor table
//!
//! The table is a fixed-size array in static storage. It never grows, and it never allocates for
//! its own bookkeeping. This is deliberate:
//!
//! - A homebrew binary opens a handful of descriptors, not thousands. A growable table would add a
//!   failure mode and an allocator dependency to buy capacity nobody uses.
//! - The C callers hold a raw pointer to a descriptor's header (see below). A table that could
//!   reallocate would invalidate those pointers. A static array cannot move, so the pointers stay
//!   valid for the lifetime of the process.
//!
//! Descriptors 0, 1 and 2 are occupied from the start, so that stdin, stdout and stderr are usable
//! before anything has been opened. Allocation of a new descriptor scans upward for the lowest
//! free slot, which is the behaviour C programs assume even though nothing on this platform relies
//! on it.
//!
//! ### Opening runs in two steps
//!
//! A descriptor number has to be reserved before the device can be asked to open anything, because
//! the C caller allocates the number first and dispatches the open second. That order is not this
//! crate's choice, and it turns out to be the right one anyway: opening a path can block on a
//! service for as long as it likes, and the table lock must not be held across it.
//!
//! So a descriptor exists in one of two conditions, and the pair is what the table stores:
//!
//! - **Named, owning nothing.** The slot records a device and no more. This is the finished state
//!   for a stream, and the intermediate state for a path.
//! - **Named and owning a file.** The slot additionally holds the [`device::File`] the device
//!   produced, and every operation goes to that object.
//!
//! There is no third state for a half-open descriptor, because there is nothing to represent: the
//! file is constructed with no lock held and only then handed to the table, so the slot never
//! contains an object that is not yet valid. If the open fails, the C caller releases the number
//! and the slot goes straight back to free.
//!
//! ### Nothing that blocks runs under the table lock
//!
//! Dropping an open file is not free. It can close a kernel handle, tear down a session, or free
//! memory. Doing that while holding the table lock risks a long hold at best and a deadlock at
//! worst, because a device's close path may reach back into the table. The same is true of an
//! ordinary write, which may sit on a storage service indefinitely.
//!
//! Two rules follow, and every operation obeys them:
//!
//! - **Resolve, release, then call.** An operation clones the handle to the open file out of the
//!   slot under the lock, drops the lock, and only then locks the file itself. Two threads writing
//!   the same descriptor serialize on that second lock, not on the table.
//! - **Displace under the lock, drop outside it.** Anything removed from a slot travels out of the
//!   locked region before it is dropped. Because the handle is reference counted, a close that
//!   races an in-flight write frees the descriptor number immediately and releases the file when
//!   the write finishes with it.
//!
//! ## Per-descriptor state for a C device
//!
//! A device registered from C declares how many bytes of private state each of its descriptors
//! needs, and reaches them through the descriptor header. That is still honoured, and it is
//! separate from the [`device::File`] a Rust device produces: a C device's state is bytes this
//! crate allocates and never interprets, while a Rust device's file is an object this crate owns
//! and calls.
//!
//! A Rust device declares no such state, so its descriptors allocate nothing beyond the file
//! itself, and a device with no files at all, such as a console, allocates nothing whatsoever.
//!
//! ## The C boundary
//!
//! The C consumers of these tables are not replaced. Roughly nineteen translation units in
//! `libsysbase` implement the POSIX entry points (`_write_r`, `_read_r`, `_open_r`, `_lseek_r`,
//! `_stat_r` and the rest) and they continue to be linked from the archive unchanged. Each of them
//! looks up a descriptor, reads the device out of it, and calls through the device's operations.
//!
//! That works because this crate exports the same symbols with the same layouts. Two consequences
//! follow, and both are load-bearing:
//!
//! - The descriptor header exposed to C has a fixed layout that cannot be reordered, and lookups
//!   must return a pointer into stable storage.
//! - The device operation table has a fixed layout with a large number of optional function
//!   pointers, and the registry is an array of pointers to it that C indexes directly.
//!
//! One shared table serves every Rust device, so a shim cannot tell from the function pointer which
//! device it was called for. It recovers that from its arguments, and where it comes from differs
//! by operation: a descriptor number for the per-descriptor operations, the path itself for the
//! per-path ones, and the iterator's own private state for a directory walk. A directory needs the
//! third because it has no descriptor number to be found by; the C caller allocates one iterator per
//! open directory, so declaring how much state to allocate behind it is enough to make the walk
//! findable, with no table and no bound on how many may be open.
//!
//! Errors reach C by writing an error number into the calling thread's reentrancy structure, which
//! is otherwise opaque to this crate. Most entry points are handed that structure directly. The
//! few that are not, because their C prototypes take no such argument, reach it through the
//! calling thread's variables instead, which is where the thread runtime records it.
//!
//! ## Symbol replacement
//!
//! The override script aliases each C name to its replacement here, which makes every reference
//! resolve to this crate no matter who emitted it.
//!
//! The C translation units are still pulled out of the archive, because unrelated members reference
//! symbols they define. What keeps them from mattering is that the aliases leave every one of their
//! sections unreferenced, so section garbage collection drops the lot: the C descriptor array, its
//! lock, its standard descriptor headers, and all of its code. Only zero-length unwind entries
//! survive. The build compiles with per-function and per-data sections for exactly this reason.
//!
//! So the override script must claim all six globals of the handle table and all seven of the
//! device registry, including the two that are data rather than functions. A symbol left unclaimed
//! keeps its C definition reachable, and then two implementations are live at once over separate
//! state, which is far worse than either alone. Adding a symbol to this crate without adding it to
//! the script leaves the C version in place silently; the reverse is a link error.
//! `sys_fd_override.ld` and [`ffi`] must be kept in step.
//!
//! ## Layout
//!
//! Everything here is shaped by `libsysbase`: the descriptor header, the device operation table
//! and the reentrancy structure are all its inventions rather than standard C, and the table
//! itself stores the C header inline because lookups hand out pointers into it. So the whole
//! surface lives under one subtree named for the archive it replaces, and none of it is compiled
//! unless the C-facing surface is.
//!
//! The structures the entry points write through, `struct stat` and `struct statvfs`, are pinned
//! field by field against the toolchain rather than transcribed from the headers, because several
//! of their fields are narrower than their names suggest. Getting one wrong would corrupt a
//! caller's stack rather than fail a test.
//!
//! ## What this crate does not do
//!
//! Descriptor duplication is not implemented. The symbols exist because the translation unit
//! defines them and the link would otherwise fail, but they report failure. The machinery it needs
//! is now mostly in place, since an open file is already held behind a reference-counted handle
//! that several slots could share and that is already dropped outside the table lock. What is
//! missing is the C side: two descriptors sharing one file would have to agree on the header the C
//! callers hold, and on which of them the `refcount` field describes.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The `alloc` crate backs the open files and directory walks a device produces, and the
// per-descriptor state a C device asks for by declaring a size.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

pub mod device;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod registry;
pub mod table;
