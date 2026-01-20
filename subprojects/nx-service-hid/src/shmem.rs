//! Shared memory layout and access for HID service.

pub mod layout;
pub mod lifo;
pub mod types;

pub use layout::HidSharedMemory;
pub use lifo::{HidCommonLifoHeader, get_states};
pub use types::*;
