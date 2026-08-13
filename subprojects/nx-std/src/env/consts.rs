//! Facts about the platform this crate was compiled for, as `std::env::consts`
//! states them.
//!
//! Every value here is fixed by the target specification, so it is a constant
//! rather than something to ask the system at run time.

/// The CPU architecture, as `std::env::consts::ARCH` reports it.
pub const ARCH: &str = "aarch64";

/// The operating system, as `std::env::consts::OS` reports it.
pub const OS: &str = "horizon";

/// The operating-system family, as `std::env::consts::FAMILY` reports it.
///
/// Empty, because the target belongs to none. Horizon is not a Unix, and the
/// target specification declares no family, so `cfg(unix)` and `cfg(windows)`
/// are both false for every crate in this workspace.
pub const FAMILY: &str = "";

// `EXE_SUFFIX`, `EXE_EXTENSION` and the `DLL_*` constants are deliberately
// absent. A Switch executable is an NRO, an NSO or a KIP depending on which
// runtime entry crate the binary linked, so naming one suffix here would state
// a fact this layer cannot know and must not guess (see `docs/code/crates-rt.md`).
