//! Thread Implementation

use nx_svc::thread as svc;

mod activity;
mod context;
mod exit;
mod info;
mod sleep;
mod stackmem;
mod wait;

pub use activity::*;
pub use context::*;
pub use exit::*;
pub use info::*;
pub use sleep::*;
pub use stackmem::*;
pub use wait::*;

/// Gets the current processor/CPU core number.
///
/// Returns the ID of the CPU core that the current thread is running on.
/// This wraps the `svcGetCurrentProcessorNumber` system call.
pub fn get_current_cpu() -> u32 {
    svc::get_current_processor_number()
}
