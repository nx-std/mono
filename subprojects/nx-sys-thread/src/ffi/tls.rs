//! FFI bindings for the `nx-sys-thread` crate
//!
//! # References
//! - [switchbrew/libnx: switch/arm/tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)
//! - [switchbrew/libnx: internal.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/internal.h)

use crate::tls_block;

/// Returns the start offset (in bytes) of the initialised TLS data (`.tdata`/`.tbss`) within a
/// thread's TLS block. Mirrors the behaviour of `getTlsStartOffset()` from the original C
/// implementation.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread__get_tls_start_offset() -> usize {
    tls_block::tdata::start_offset()
}
