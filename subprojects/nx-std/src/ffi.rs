//! FFI exports re-exported from dependent crates.
//!
//! This module ensures that all `#[no_mangle]` FFI symbols from dependent crates
//! are included in the nx-std staticlib by explicitly referencing them.

// Re-export ffi modules from dependent crates to force their symbols
// to be included in the staticlib.

#[cfg(feature = "alloc")]
pub use nx_alloc::ffi as alloc;
#[cfg(feature = "rand")]
pub use nx_rand::ffi as rand;
#[cfg(feature = "rt")]
pub use nx_rt::ffi as rt;
#[cfg(feature = "sf")]
pub use nx_sf::ffi as sf;
#[cfg(feature = "sync")]
pub use nx_std_sync::ffi as sync;
#[cfg(feature = "svc")]
pub use nx_svc::ffi as svc;
#[cfg(feature = "sys-mem")]
pub mod sys_mem {
    pub use nx_sys_mem::{shmem::ffi as shmem, tmem::ffi as tmem, vmm::ffi as vmm};
}
#[cfg(feature = "sys-sync")]
pub use nx_sys_sync::ffi as sys_sync;
#[cfg(feature = "sys-thread")]
pub use nx_sys_thread::ffi as sys_thread;
#[cfg(feature = "sys-thread-tls")]
pub use nx_sys_thread_tls::ffi as sys_thread_tls;
#[cfg(feature = "time")]
pub use nx_time::ffi as time;
