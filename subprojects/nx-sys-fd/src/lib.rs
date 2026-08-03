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
//! ### Slot states
//!
//! A slot is in one of three states, and the third is the one that carries its weight:
//!
//! - **Available**: nothing here, the number is free.
//! - **Reserved**: the number is taken, but the backing object does not exist yet.
//! - **Occupied**: the number is taken and the object is ready to use.
//!
//! `Reserved` exists because opening a device is not instantaneous. The device may need to
//! allocate per-descriptor state, talk to a service, or fail partway through. None of that can
//! happen while the table lock is held, because a device's open or close path may block or may
//! itself want a descriptor. But the descriptor number cannot be handed out to another thread in
//! the meantime either.
//!
//! So opening runs in two phases. First, claim a slot and mark it `Reserved`, under the lock.
//! Then release the lock and construct the object. Then take the lock again and either fill the
//! slot, promoting it to `Occupied`, or roll it back to `Available` if construction failed.
//!
//! The rollback is not left to the caller to remember. A reservation is represented by a guard
//! value that releases the slot when dropped, so an early return or a `?` cannot strand a slot in
//! `Reserved` forever. Filling the slot consumes the guard.
//!
//! Without this state, a partially opened descriptor has no representation at all: the slot would
//! have to hold an object that is not yet valid, which is exactly the pointer-shaped hole the C
//! implementation leaves when it allocates and populates in a single step while holding its lock.
//!
//! ### Closing happens outside the lock
//!
//! Dropping an open object is not free. It can close a kernel handle, tear down a session, or free
//! memory. Doing that while holding the table lock risks a long hold at best and a deadlock at
//! worst, because a device's close path may reach back into the table.
//!
//! Every operation that displaces an object therefore removes it from the slot under the lock,
//! releases the lock, and only then drops it. This applies to closing a descriptor and to any
//! future operation that replaces one.
//!
//! ## Per-descriptor device state
//!
//! A device declares how many bytes of private state each of its open descriptors needs. Most
//! devices need none: a console writes to a framebuffer that is global to the device, so its
//! descriptors carry nothing. A filesystem needs a seek position and an open file object per
//! descriptor.
//!
//! Devices that declare no per-descriptor state allocate nothing, which keeps the common path free
//! of the heap entirely. Devices that do declare state get exactly that many bytes, allocated when
//! the descriptor is opened and released when it is closed.
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
//! ## What this crate does not do
//!
//! Descriptor duplication is not implemented. The symbols exist because the translation unit
//! defines them and the link would otherwise fail, but they report failure. Implementing them
//! requires sharing one object between several slots, which means reference counting the object
//! and replacing a slot's contents without dropping the old object under the lock.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The `alloc` crate backs the per-descriptor state a C device asks for by declaring a size.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

pub mod device;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod registry;
pub mod table;
