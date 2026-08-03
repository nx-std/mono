//! The descriptor header C reads, and the tag that gives a shim its identity.
//!
//! `__get_handle` hands C a pointer to a header describing an open descriptor. The layout is fixed
//! by the C standard library and the pointer has to stay valid while the descriptor is open, so
//! the headers live in a static array that never moves and are refreshed from the Rust table on
//! each lookup. The table remains the single source of truth; these are a projection of it.
//!
//! The header also solves an identity problem. C dispatches with
//! `devoptab_list[dev]->write_r(r, handle->file_struct, ...)`, which carries no indication of
//! which device is being addressed: the C design relies on the function pointer itself being
//! device-specific. A Rust device instead shares one set of shims, so the shims recover the
//! descriptor from `file_struct`, which for a Rust device points at that descriptor's entry in
//! [`FD_TAGS`].

use core::{
    cell::UnsafeCell,
    ffi::c_void,
};

use crate::table::{
    Fd,
    MAX_FD,
};

/// Header describing one open descriptor, as the C standard library reads it.
///
/// Mirrors `__handle` from `sys/iosupport.h`.
#[derive(Debug)]
#[repr(C)]
pub struct Handle {
    /// Registry slot of the device backing this descriptor.
    pub device: u32,
    /// Number of descriptors sharing this header.
    ///
    /// Always 1: duplication is not implemented, so no two descriptors share a header.
    pub refcount: u32,
    /// What the device is handed as its per-descriptor state.
    pub file_struct: *mut c_void,
}

// The C entry points index these fields by offset. Pin the layout so a change to the struct is a
// build failure here rather than memory corruption at the first `printf`.
static_assertions::assert_eq_size!(Handle, [u64; 2]);
const _: () = {
    assert!(core::mem::offset_of!(Handle, device) == 0);
    assert!(core::mem::offset_of!(Handle, refcount) == 4);
    assert!(core::mem::offset_of!(Handle, file_struct) == 8);
};

/// One descriptor number per entry, so `FD_TAGS[n] == n`.
///
/// A shim reads the descriptor back out of the `file_struct` pointer it is handed, which is how one
/// shared set of shims serves every Rust device.
static FD_TAGS: [u32; MAX_FD] = {
    let mut tags = [0u32; MAX_FD];
    let mut fd = 0;
    while fd < MAX_FD {
        tags[fd] = fd as u32;
        fd += 1;
    }
    tags
};

static HANDLES: Handles = Handles(UnsafeCell::new({
    let mut handles = [const {
        Handle {
            device: 0,
            refcount: 1,
            file_struct: core::ptr::null_mut(),
        }
    }; MAX_FD];
    let mut fd = 0;
    while fd < MAX_FD {
        handles[fd].device = 0;
        fd += 1;
    }
    handles
}));

/// Refreshes the header for `fd` and returns a pointer C may hold while the descriptor is open.
///
/// `file_struct` is what the device will be handed: the descriptor's tag for a Rust device, whose
/// shims recover the descriptor from it, or the device's own allocated state for a C device, which
/// dispatches to its own table and expects nothing else.
pub fn project(fd: Fd, device: u32, c_state: *mut u8) -> *mut Handle {
    let number = fd.number();
    if number >= MAX_FD {
        return core::ptr::null_mut();
    }

    let file_struct = if c_state.is_null() {
        core::ptr::from_ref(&FD_TAGS[number]).cast_mut().cast()
    } else {
        c_state.cast()
    };

    // SAFETY: `number` is in range, the array is static, and C reads one descriptor's header at a
    // time through the pointer returned here.
    let handle = unsafe { &mut (*HANDLES.0.get())[number] };
    handle.device = device;
    handle.refcount = 1;
    handle.file_struct = file_struct;

    core::ptr::from_mut(handle)
}

/// Recovers the descriptor a shim was invoked for from the state pointer it was handed.
///
/// Returns `None` when the pointer does not address [`FD_TAGS`], which means the device was
/// registered from C and dispatches through its own table rather than through a shim.
pub fn fd_from_state(state: *mut c_void) -> Option<Fd> {
    if state.is_null() {
        return None;
    }

    let base = FD_TAGS.as_ptr();
    // SAFETY: both pointers are compared as addresses only; neither is dereferenced.
    let offset = (state as usize).checked_sub(base as usize)?;
    if offset % size_of::<u32>() != 0 {
        return None;
    }

    let number = offset / size_of::<u32>();
    if number >= MAX_FD {
        return None;
    }

    // SAFETY: `number` was just checked against `MAX_FD` above.
    Some(Fd::from_number_unchecked(number))
}

/// Storage for the headers handed to C.
struct Handles(UnsafeCell<[Handle; MAX_FD]>);

// SAFETY: an entry is only written by `project`, which C reaches one descriptor at a time, and the
// array is never moved.
unsafe impl Sync for Handles {}
