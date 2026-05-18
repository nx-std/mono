//! `libnx` symbol-override FFI for `nx-rt-nso`.
//!
//! Holds the `__nx_rt_nso__libnx_*` symbols that redirect the NSO-specific
//! `libnx` runtime entry points — the `pm`-handoff environment setup, the
//! `__argdata__` command-line (`argv`) path, and the Application Manager
//! (applet) bring-up — to this crate. The override aliases live in the
//! `overrides/rt_nso_libnx_core.ld` and `overrides/rt_nso_libnx_service_applet.ld` fragments.
//!
//! The kind-agnostic runtime entry points (heap init, main-thread TLS, the
//! environment read accessors, the HOS-version API, the Service Manager set)
//! are intentionally absent: `nx-rt-core` owns those, along with its
//! `rt_nso_libnx_core.ld` overrides.

mod applet;
mod argv;
mod env;

// Called by `argv::setup()` after parsing the `__argdata__` command line.
pub(crate) use self::argv::set_system_argv;
