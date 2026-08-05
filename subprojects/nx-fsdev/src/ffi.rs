//! The C-facing surface, replacing libnx's `fsdev*`.
//!
//! Every symbol here stands in for one libnx `fs_dev.c` export, and `fsdev_override.ld`
//! aliases the C name to it. The two files are one unit: a symbol added here without an alias is
//! never reached, and an alias without a symbol is a link error.
//!
//! The replacement has to be total for the same reason the `fs*` surface's is. A `fsdev*` call
//! left to libnx runs against a `g_fsSrv` that was never initialized, and libnx's session manager
//! does not fail on it: it waits forever. So the commands this crate does not implement are
//! aliased to panicking stubs rather than left behind: a panic names what is missing, a hang names
//! nothing.

pub(crate) mod common;
pub mod libnx;
