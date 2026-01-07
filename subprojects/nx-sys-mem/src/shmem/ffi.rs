//! C FFI bindings for shared memory operations
//!
//! These bindings provide `#[no_mangle]` C-callable functions whose
//! signatures align with the declarations in `nx_shmem.h`.

use core::{
    ffi::c_void,
    ptr::{self, NonNull},
};

use nx_svc::{
    error::{KernelError, ToRawResultCode},
    mem::shmem::Handle,
    raw::{Handle as RawHandle, INVALID_HANDLE},
};

use super::sys::{self, Mapped, Unmapped};

/// Shared memory object (C-compatible wrapper)
#[repr(C)]
struct SharedMemory {
    handle: RawHandle,
    size: usize,
    perm: u32,
    map_addr: *mut c_void,
}

/// Creates a shared memory object.
///
/// Corresponds to `shmemCreate()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_create(
    s: *mut SharedMemory,
    size: usize,
    local_perm: u32,
    remote_perm: u32,
) -> u32 {
    let Some(s) = NonNull::new(s) else {
        return KernelError::InvalidPointer.to_rc();
    };

    match unsafe {
        sys::create(
            size,
            sys::LocalPermissions::from_bits_retain(local_perm),
            sys::RemotePermissions::from_bits_retain(remote_perm),
        )
    } {
        Ok(unmapped) => {
            let sm = SharedMemory {
                handle: unmapped.handle().to_raw(),
                size: unmapped.size(),
                perm: unmapped.perm().bits(),
                map_addr: ptr::null_mut(),
            };
            unsafe { s.write(sm) };

            0
        }
        Err(err) => err.into_rc(),
    }
}

/// Loads a remote shared memory object (pure struct copy).
///
/// Corresponds to `shmemLoadRemote()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_load_remote(
    s: *mut SharedMemory,
    handle: u32,
    size: usize,
    perm: u32,
) {
    let Some(s) = NonNull::new(s) else {
        return;
    };

    let sm = SharedMemory {
        handle,
        size,
        perm,
        map_addr: ptr::null_mut(),
    };
    unsafe { s.write(sm) };
}

/// Maps a shared memory object into the current process.
///
/// Corresponds to `shmemMap()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_map(s: *mut SharedMemory) -> u32 {
    let Some(mut s) = NonNull::new(s) else {
        return KernelError::InvalidPointer.to_rc();
    };
    let sm = unsafe { s.as_ref() };

    // Prevent double-mapping (behaves like libnx).
    if !sm.map_addr.is_null() {
        return LIBNX_ERR_ALREADY_MAPPED;
    }

    let unmapped = unsafe {
        {
            sys::SharedMemory::<Unmapped>::from_parts(
                Handle::from_raw(sm.handle),
                sm.size,
                sys::Permissions::from_bits_retain(sm.perm),
            )
        }
    };
    match unsafe { sys::map(unmapped) } {
        Ok(mapped) => {
            let sm = unsafe { s.as_mut() };

            // Update the shared memory object with the mapped address.
            sm.map_addr = mapped.addr().unwrap_or(ptr::null_mut());

            0
        }
        Err(err) => match err {
            sys::MapError::VirtAddressAllocFailed => LIBNX_ERR_OUT_OF_MEMORY,
            sys::MapError::Svc(svc_err) => svc_err.to_rc(),
        },
    }
}

/// Unmaps a shared memory object.
///
/// Corresponds to `shmemUnmap()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_unmap(s: *mut SharedMemory) -> u32 {
    let Some(mut s) = NonNull::new(s) else {
        return KernelError::InvalidPointer.to_rc();
    };
    let sm = unsafe { s.as_mut() };

    let Some(map_addr) = NonNull::new(sm.map_addr) else {
        // Nothing mapped – treat as success per libnx semantics.
        return 0;
    };

    let mapped = unsafe {
        sys::SharedMemory::<Mapped>::from_parts(
            Handle::from_raw(sm.handle),
            sm.size,
            sys::Permissions::from_bits_retain(sm.perm),
            map_addr,
        )
    };

    match unsafe { sys::unmap(mapped) } {
        Ok(_unmapped) => {
            sm.map_addr = ptr::null_mut();
            0
        }
        Err(err) => err.reason.to_rc(),
    }
}

/// Returns the mapped address of the shared memory object.
///
/// Corresponds to `shmemGetAddr()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_get_addr(s: *mut SharedMemory) -> *mut c_void {
    let Some(s) = NonNull::new(s) else {
        return ptr::null_mut();
    };
    let sm = unsafe { s.as_ref() };

    sm.map_addr
}

/// Frees resources (unmap + close).
///
/// Corresponds to `shmemClose()` in `shmem.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_mem__shmem_close(s: *mut SharedMemory) -> u32 {
    let Some(mut s) = NonNull::new(s) else {
        return KernelError::InvalidPointer.to_rc();
    };
    let sm = unsafe { s.as_ref() };

    // If mapped, unmap first.
    if !sm.map_addr.is_null() {
        let rc = unsafe { __nx_sys_mem__shmem_unmap(s.as_ptr()) };
        if rc != 0 {
            return rc;
        }
    }

    let Some(handle) = Handle::new(sm.handle) else {
        // Handle already closed
        return 0;
    };

    let unmapped = unsafe {
        {
            sys::SharedMemory::<Unmapped>::from_parts(
                handle,
                sm.size,
                sys::Permissions::from_bits_retain(sm.perm),
            )
        }
    };
    match unsafe { sys::close(unmapped) } {
        Ok(()) => {
            let sm = unsafe { s.as_mut() };

            // Clear the handle to prevent double-close.
            sm.handle = INVALID_HANDLE;

            0
        }
        Err(err) => err.reason.to_rc(),
    }
}

// Helper: builds a libnx-style result-code from a description value.
#[inline]
const fn libnx_rc(desc: u32) -> u32 {
    const MODULE_LIBNX: u32 = 345; // 0x159
    (MODULE_LIBNX & 0x1FF) | (desc << 9)
}

const LIBNX_ERR_OUT_OF_MEMORY: u32 = libnx_rc(2);
const LIBNX_ERR_ALREADY_MAPPED: u32 = libnx_rc(3);
