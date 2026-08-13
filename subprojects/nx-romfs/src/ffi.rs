//! The C-facing surface, replacing libnx's `romfs*`.
//!
//! Every symbol here stands in for one libnx `romfs_dev.c` export, and `romfs_override.ld` aliases
//! the C name to it. The two files are one unit: a symbol added here without an alias is never
//! reached, and an alias without a symbol is a link error.
//!
//! One export is deliberately absent. `romfsMountSelf` asks which output kind the process is before
//! it can pick a source, and that question is answered above this crate by which runtime crate the
//! binary links. Its alias therefore lives in the `nx-rt-*` override fragments, pointing at a symbol
//! each entry crate defines for itself.

pub(crate) mod common;
pub mod libnx;
