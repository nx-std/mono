//! Filesystem (`fsp-srv`) service FFI.
//!
//! libnx keeps its `fsp-srv` session in a file-local `g_fsSrv`, so a linker
//! script cannot redirect it: `static` gives it internal linkage, and every
//! reference inside `fs.c` was already bound to that definition at compile
//! time. What makes the override work anyway is that *every* reader of
//! `g_fsSrv` is itself an `fs*` function. Redirect the whole set fsdev calls
//! and the C global is simply never read.
//!
//! That is why this module is all-or-nothing, and why it covers the whole
//! command surface rather than the part fsdev happens to call. A command left
//! to libnx does not fail cleanly: `_fsObjectIsChild` compares
//! `g_fsSrv.session` against itself, `0 == 0` holds, so the call takes the
//! session-pool path into `sessionmgrAttachClient`, finds an empty free mask
//! and parks forever on a condvar nothing will signal. The commands not
//! implemented yet are therefore aliased to stubs that panic naming the
//! command, which is a diagnosable failure rather than a hang.
//!
//! The `fsdev*`, `fsldr*` and `fspr*` families are deliberately left alone:
//! fsdev is the devoptab layer that *calls* these commands rather than being
//! one of them, and fs:ldr and fs:pr are separate services holding their own
//! sessions, which this override does not touch.
//!
//! # Object ownership across the boundary
//!
//! C holds a `Service` per filesystem, file and directory, and decides when
//! each dies (`fsFsClose` and friends). The Rust wrappers are RAII with a
//! lifetime tied to the session, so each entry point here rebuilds the wrapper
//! from the stored object id, runs one command, and hands the close obligation
//! straight back. Only the `*Close` entry points let the wrapper drop, which is
//! what sends the close.
mod device_operator;
mod event_notifier;
mod filesystem;
mod global;
mod idirectory;
mod ifile;
mod ifilesystem;
mod istorage;
mod lifecycle;
mod savedata;
mod savedata_info_reader;
mod support;
