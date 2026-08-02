//! Borrowed buffer wrappers for HIPC descriptor builders.
//!
//! Each wrapper models one HIPC wire role as a `ptr+len` view with a borrow
//! lifetime, mirroring `std::io::IoSlice<'a>` / `IoSliceMut<'a>` for the
//! `iovec`-shaped read/write APIs in std. Builder methods accept these
//! wrappers in place of raw pointers so the kernel's read or write through
//! the buffer during `SendSyncRequest` is witnessed by the borrow checker
//! for the duration of the call.
//!
//! Read-from-buffer roles (Type-A send-buffer, Type-X send-static) take
//! [`InputBuffer`] / [`InPointer`]; write-into-buffer roles (Type-B
//! recv-buffer, Type-W exch-buffer, Type-C recv-list) take [`OutputBuffer`]
//! / [`InOutBuffer`] / [`OutPointer`] so `&mut` exclusivity prevents two
//! descriptors in the same request from aliasing the same memory.
//!
//! Each wrapper exposes a safe [`new`](InputBuffer::new) constructor from a
//! slice and an `unsafe` [`from_raw_parts`](InputBuffer::from_raw_parts)
//! constructor for FFI shims that hold only a `(ptr, len)` pair and uphold
//! the validity contract out-of-band.

use super::wire::BufferMode;

/// Borrowed input buffer for Type-A send-buffer / Type-W exch-buffer (input
/// half) and auto-buffer input descriptors. The kernel reads through this
/// buffer during the syscall.
#[derive(Debug, Clone, Copy)]
pub struct InputBuffer<'a> {
    slice: &'a [u8],
    mode: BufferMode,
}

impl<'a> InputBuffer<'a> {
    /// Wraps a borrowed slice as a kernel-readable input buffer.
    #[inline]
    pub const fn new(slice: &'a [u8], mode: BufferMode) -> Self {
        Self { slice, mode }
    }

    /// FFI escape hatch for callers that hold only a `(ptr, len)` pair.
    ///
    /// # Safety
    /// - `ptr` must be valid for reads of `len` bytes for the duration of
    ///   the `SendSyncRequest` that consumes the request this descriptor is
    ///   attached to.
    /// - The referenced memory must not be mutated, freed, or remapped
    ///   between this call and that syscall returning.
    /// - The buffer must not alias any output buffer attached to the same
    ///   request.
    /// - `ptr` and `len` must together satisfy the requirements of
    ///   [`core::slice::from_raw_parts`].
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: *const u8, len: usize, mode: BufferMode) -> Self {
        // SAFETY: caller upholds the validity, exclusivity, and slice-shape
        // contract documented above.
        let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
        Self { slice, mode }
    }

    /// Borrows the underlying byte slice.
    #[inline]
    pub const fn as_slice(&self) -> &'a [u8] {
        self.slice
    }

    /// Returns the buffer transfer mode.
    #[inline]
    pub const fn mode(&self) -> BufferMode {
        self.mode
    }
}

/// Borrowed output buffer for Type-B recv-buffer and auto-buffer output
/// descriptors. The kernel writes through this buffer during the syscall.
#[derive(Debug)]
pub struct OutputBuffer<'a> {
    slice: &'a mut [u8],
    mode: BufferMode,
}

impl<'a> OutputBuffer<'a> {
    /// Wraps a borrowed mutable slice as a kernel-writable output buffer.
    #[inline]
    pub const fn new(slice: &'a mut [u8], mode: BufferMode) -> Self {
        Self { slice, mode }
    }

    /// FFI escape hatch for callers that hold only a `(ptr, len)` pair.
    ///
    /// # Safety
    /// - `ptr` must be valid for reads and writes of `len` bytes for the
    ///   duration of the `SendSyncRequest` that consumes the request this
    ///   descriptor is attached to.
    /// - The referenced memory must be exclusively borrowed: no other
    ///   reference, descriptor, or thread may access it between this call
    ///   and that syscall returning.
    /// - `ptr` and `len` must together satisfy the requirements of
    ///   [`core::slice::from_raw_parts_mut`].
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: *mut u8, len: usize, mode: BufferMode) -> Self {
        // SAFETY: caller upholds the validity, exclusivity, and slice-shape
        // contract documented above.
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        Self { slice, mode }
    }

    /// Borrows the underlying byte slice.
    #[inline]
    pub const fn as_slice(&self) -> &[u8] {
        self.slice
    }

    /// Borrows the underlying byte slice mutably.
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [u8] {
        self.slice
    }

    /// Returns the buffer transfer mode.
    #[inline]
    pub const fn mode(&self) -> BufferMode {
        self.mode
    }
}

/// Borrowed exchange buffer for Type-W exch-buffer descriptors. The kernel
/// reads then writes through this buffer during the syscall.
#[derive(Debug)]
pub struct InOutBuffer<'a> {
    slice: &'a mut [u8],
    mode: BufferMode,
}

impl<'a> InOutBuffer<'a> {
    /// Wraps a borrowed mutable slice as a kernel-read-then-writable buffer.
    #[inline]
    pub const fn new(slice: &'a mut [u8], mode: BufferMode) -> Self {
        Self { slice, mode }
    }

    /// FFI escape hatch for callers that hold only a `(ptr, len)` pair.
    ///
    /// # Safety
    /// - `ptr` must be valid for reads and writes of `len` bytes for the
    ///   duration of the `SendSyncRequest` that consumes the request this
    ///   descriptor is attached to.
    /// - The referenced memory must be exclusively borrowed: no other
    ///   reference, descriptor, or thread may access it between this call
    ///   and that syscall returning.
    /// - `ptr` and `len` must together satisfy the requirements of
    ///   [`core::slice::from_raw_parts_mut`].
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: *mut u8, len: usize, mode: BufferMode) -> Self {
        // SAFETY: caller upholds the validity, exclusivity, and slice-shape
        // contract documented above.
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        Self { slice, mode }
    }

    /// Borrows the underlying byte slice.
    #[inline]
    pub const fn as_slice(&self) -> &[u8] {
        self.slice
    }

    /// Borrows the underlying byte slice mutably.
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [u8] {
        self.slice
    }

    /// Returns the buffer transfer mode.
    #[inline]
    pub const fn mode(&self) -> BufferMode {
        self.mode
    }
}

/// Borrowed input pointer for Type-X send-static descriptors. The kernel
/// reads through this buffer during the syscall.
#[derive(Debug, Clone, Copy)]
pub struct InPointer<'a> {
    slice: &'a [u8],
}

impl<'a> InPointer<'a> {
    /// Wraps a borrowed slice as a kernel-readable send-static pointer.
    #[inline]
    pub const fn new(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    /// FFI escape hatch for callers that hold only a `(ptr, len)` pair.
    ///
    /// # Safety
    /// - `ptr` must be valid for reads of `len` bytes for the duration of
    ///   the `SendSyncRequest` that consumes the request this descriptor is
    ///   attached to.
    /// - The referenced memory must not be mutated, freed, or remapped
    ///   between this call and that syscall returning.
    /// - The buffer must not alias any output buffer attached to the same
    ///   request.
    /// - `ptr` and `len` must together satisfy the requirements of
    ///   [`core::slice::from_raw_parts`].
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: *const u8, len: usize) -> Self {
        // SAFETY: caller upholds the validity, exclusivity, and slice-shape
        // contract documented above.
        let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
        Self { slice }
    }

    /// Borrows the underlying byte slice.
    #[inline]
    pub const fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}

/// Borrowed output pointer for Type-C recv-list entries. The kernel writes
/// through this buffer during the syscall.
#[derive(Debug)]
pub struct OutPointer<'a> {
    slice: &'a mut [u8],
}

impl<'a> OutPointer<'a> {
    /// Wraps a borrowed mutable slice as a kernel-writable recv-list entry.
    #[inline]
    pub const fn new(slice: &'a mut [u8]) -> Self {
        Self { slice }
    }

    /// FFI escape hatch for callers that hold only a `(ptr, len)` pair.
    ///
    /// # Safety
    /// - `ptr` must be valid for writes of `len` bytes for the duration of
    ///   the `SendSyncRequest` that consumes the request this descriptor is
    ///   attached to.
    /// - The referenced memory must be exclusively borrowed: no other
    ///   reference, descriptor, or thread may access it between this call
    ///   and that syscall returning.
    /// - `ptr` and `len` must together satisfy the requirements of
    ///   [`core::slice::from_raw_parts_mut`].
    #[inline]
    pub const unsafe fn from_raw_parts(ptr: *mut u8, len: usize) -> Self {
        // SAFETY: caller upholds the validity, exclusivity, and slice-shape
        // contract documented above.
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        Self { slice }
    }

    /// Borrows the underlying byte slice.
    #[inline]
    pub const fn as_slice(&self) -> &[u8] {
        self.slice
    }

    /// Borrows the underlying byte slice mutably.
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [u8] {
        self.slice
    }
}
