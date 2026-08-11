//! # nx-nv
//!
//! Memory objects on the NV driver, built on the driver session that
//! [`nx_service_nv`] establishes.
//!
//! ## What this crate is for
//!
//! A buffer the CPU drew into becomes a buffer something else can display by
//! being handed to the driver, which pins its pages and names it with an id
//! any process can be told. That exchange is what [`map`] implements, and it
//! is the piece the display stack stands on: a framebuffer is a buffer with a
//! memory object over it and an id handed to the compositor.
//!
//! ## No process-wide state lives here
//!
//! Everything in this crate takes the driver session and the device it works
//! through as parameters. A process has exactly one driver session and one
//! sensible moment to open the device, but neither fact belongs here: the
//! runtime owns the session's lifetime, and a singleton pinned in this crate
//! would be a second opinion about it. Callers that need one keep it beside
//! the session it borrows.
//!
//! That is also what makes the allocation path reachable without a console:
//! the types compose against any session, so a test can drive them directly
//! rather than through whatever global a C caller would have gone via.
//!
//! ## Layering
//!
//! ```text
//!   nx-nv               MemoryMap · MapBuffer · MapId
//!       |
//!   nx-service-nv       Open · Ioctl · Close on the driver session
//!       |
//!   NV driver           /dev/nvmap
//! ```

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod graphic_buffer;
pub mod map;

pub use self::{
    graphic_buffer::{
        ColorFormat,
        GraphicBuffer,
        GraphicBufferParams,
        Layout,
        ScanFormat,
        Surface,
        USAGE_HW_COMPOSER,
        USAGE_HW_RENDER,
        USAGE_HW_TEXTURE,
    },
    map::{
        AdoptMapError,
        BorrowedMapDevice,
        CacheAttrError,
        CreateMapError,
        MapAlign,
        MapAlignError,
        MapBuffer,
        MapBufferError,
        MapHandle,
        MapId,
        MapKind,
        MemoryMap,
        NvMapDevice,
        OpenError,
        PAGE_SIZE,
    },
};
