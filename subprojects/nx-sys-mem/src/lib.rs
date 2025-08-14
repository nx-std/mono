#![no_std]

// The `alloc` crate enables memory allocation.
extern crate alloc;
// The `nx-alloc` crate exposes the `#[global_allocator]` for the dependent crates.
extern crate nx_alloc;

pub mod buf;
pub mod shmem;
pub mod stack;
pub mod tmem;
pub mod vmm;
