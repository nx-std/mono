//! Placing an NRO in the address space of the process that will run it.

use core::{
    ffi::c_void,
    ptr::NonNull,
};

use nx_object::read::nro::Nro;
use nx_svc::{
    mem::{
        ProcessMemoryPermission,
        map_process_code_memory,
        set_process_memory_permission,
        unmap_process_code_memory,
    },
    process::Handle as ProcessHandle,
};

/// The granularity every mapping operation works in.
///
/// Addresses and sizes handed to the kernel are page aligned, so the sizes
/// derived from an image's header are rounded up to this before use.
const PAGE_SIZE: usize = 0x1000;

/// Maps the NRO image at `image` into `process` and gives its segments the
/// permissions they run under.
///
/// This is the layout half of the handover: an image is three segments plus a
/// zero-filled tail, and which of them ends up executable, read-only, or
/// writable is fixed by the format rather than by the loader. The destination
/// is chosen here too, from the virtual-memory reservation map and under its
/// lock, because a destination picked before the call could be taken by another
/// thread before the mapping is made.
///
/// The image keeps occupying `image` afterwards: the mapping is a second view
/// of those same pages, not a copy. [`MappedNro::image_size`] is what that
/// costs, which is what a loader carving a heap out of the rest needs to know.
///
/// Either everything succeeds or nothing is left mapped: a failure part-way
/// through the permission changes undoes the mapping before returning.
///
/// The virtual-memory reservation map must already be set up, since that is
/// where the destination comes from.
///
/// # Safety
///
/// `image` must be page aligned and address readable bytes this process has
/// mapped, and they must stay mapped until the returned [`MappedNro`] is passed
/// to [`unmap`](MappedNro::unmap). A pointer to a region of known length rather
/// than a slice, because the mapping is a second view of those same pages: the
/// program writes through it into its own data, so nothing may assume the
/// region is unaliased, and writing through `image` writes into running code.
///
/// `process` must be a handle to the process the mapping is made in, and not
/// the pseudo-handle for the current process, which this call rejects.
///
/// # Errors
///
/// Returns [`MapError::Malformed`] when the bytes are not an NRO whose segments
/// lie inside them, [`MapError::NoAddressSpace`] when the reservation map has no
/// free run long enough, and [`MapError::Map`] or [`MapError::Permissions`] when
/// the kernel refuses. Nothing is left mapped on any of them.
#[must_use = "the mapping stays until it is unmapped, which needs this value"]
pub unsafe fn map(process: ProcessHandle, image: NonNull<[u8]>) -> Result<MappedNro, MapError> {
    // The region is read as bytes only to measure it, and only before it is
    // mapped a second time: past this block the program's own writes reach it
    // through that mapping, so nothing may hold a slice over it.
    let layout = {
        // SAFETY: the caller vouched for the region being readable and mapped,
        // and nothing has been mapped over it yet, which is what makes this
        // slice one that may be read for the length of the parse.
        let bytes =
            unsafe { core::slice::from_raw_parts(image.cast::<u8>().as_ptr(), image.len()) };
        let nro = Nro::try_from_bytes(bytes).map_err(MapError::Malformed)?;

        Layout::of(&nro)
    };
    let source = image.cast::<c_void>();

    // The reservation map is consulted and the mapping made under one lock: a
    // destination found and then released is one another thread may take before
    // this mapping claims it.
    let base = {
        let mut virtmem = nx_sys_virtmem::virtmem::lock();
        let dst = virtmem
            .find_code_memory(layout.mapped_size, 0)
            .ok_or(MapError::NoAddressSpace)?;
        map_process_code_memory(process, dst, source, layout.mapped_size).map_err(MapError::Map)?;

        dst
    };

    let mapped = MappedNro {
        base,
        source,
        layout,
    };

    match mapped.apply_permissions(process) {
        Ok(()) => Ok(mapped),
        Err(err) => {
            // The mapping exists but part of it has no permissions, which is
            // not a state to hand back. Undoing it returns the caller to where
            // it was, and a failure to undo leaves nothing better to report
            // than the failure that got us here.
            let _ = mapped.unmap(process);
            Err(MapError::Permissions(err))
        }
    }
}

/// Error returned by [`map`].
#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// The bytes are not an NRO, or a segment runs past the end of them.
    #[error("the image is not a well-formed NRO")]
    Malformed(#[source] nx_object::read::nro::FromBytesError),
    /// No free run in the code region is long enough for the image.
    #[error("no free code region large enough for the image")]
    NoAddressSpace,
    /// The kernel refused the mapping.
    #[error("the kernel refused to map the image as code")]
    Map(#[source] nx_svc::mem::MapProcessCodeMemoryError),
    /// The kernel refused a segment's permissions, and the mapping was undone.
    #[error("the kernel refused a segment's permissions")]
    Permissions(#[source] nx_svc::mem::SetProcessMemoryPermissionError),
}

/// An NRO currently mapped into a process, and the means to take it back out.
///
/// The mapping outlives this value if it is simply dropped: unmapping names the
/// process it was made in, which no destructor here has. Pass it to
/// [`unmap`](Self::unmap) instead.
#[derive(Debug)]
pub struct MappedNro {
    base: NonNull<c_void>,
    source: NonNull<c_void>,
    layout: Layout,
}

impl MappedNro {
    /// Returns the address to enter the program at.
    ///
    /// An NRO begins with the stub the loader jumps to, so this is the base of
    /// the mapping rather than an offset read out of the header.
    pub fn entrypoint(&self) -> NonNull<c_void> {
        self.base
    }

    /// Returns how much of the image buffer the mapped program occupies.
    ///
    /// Counted from the start of the buffer through the writable segment, which
    /// is the last part of it the program uses. A loader that mapped the image
    /// into its own heap carves the program's heap out of what follows.
    pub fn image_size(&self) -> usize {
        self.layout.image_size
    }

    /// Takes the mapping back out of `process`.
    ///
    /// # Errors
    ///
    /// Returns [`UnmapError`] when the kernel refuses. Segments before the one
    /// that failed are already unmapped, so a failure leaves the program
    /// partly removed and the address space is not reusable as it was.
    pub fn unmap(self, process: ProcessHandle) -> Result<(), UnmapError> {
        for segment in self.layout.segments {
            // SAFETY: `file_off` is within the mapped range, which `Layout::of`
            // established from bounds `Nro::try_from_bytes` had already proven.
            let dst = unsafe { self.base.byte_add(segment.file_off) };
            // SAFETY: the same offset into the source range the mapping was
            // made from, which the caller of `map` vouched for and which is
            // still mapped because unmapping it is what this is doing.
            let src = unsafe { self.source.byte_add(segment.file_off) };

            unmap_process_code_memory(process, dst, src, segment.size).map_err(UnmapError)?;
        }

        Ok(())
    }

    /// Gives each segment the permissions it runs under.
    fn apply_permissions(
        &self,
        process: ProcessHandle,
    ) -> Result<(), nx_svc::mem::SetProcessMemoryPermissionError> {
        for (segment, permission) in self.layout.segments.iter().zip(SEGMENT_PERMISSIONS) {
            // SAFETY: `file_off` is within the mapped range, which `Layout::of`
            // established from bounds `Nro::try_from_bytes` had already proven.
            let addr = unsafe { self.base.byte_add(segment.file_off) };

            set_process_memory_permission(process, addr, segment.size, permission)?;
        }

        Ok(())
    }
}

/// What each of the three segments is allowed to do, in the order they appear
/// in the header.
///
/// This is the format's, not the loader's: code that runs, constants that
/// cannot be written, and the writable segment the zero-filled tail extends.
const SEGMENT_PERMISSIONS: [ProcessMemoryPermission; 3] = [
    ProcessMemoryPermission::ReadExecute,
    ProcessMemoryPermission::Read,
    ProcessMemoryPermission::ReadWrite,
];

/// Where each segment sits and how far the whole image reaches, in the page
/// units the kernel works in.
#[derive(Debug, Clone, Copy)]
struct Layout {
    segments: [Segment; 3],
    mapped_size: usize,
    image_size: usize,
}

impl Layout {
    /// Measures `nro`, rounding the sizes the kernel is given up to a page.
    fn of(nro: &Nro<'_>) -> Self {
        let header = nro.header();
        let bss_size = header.bss_size.get() as usize;

        let mut segments = [Segment {
            file_off: 0,
            size: 0,
        }; 3];
        for (segment, raw) in segments.iter_mut().zip(header.segments.iter()) {
            segment.file_off = raw.file_off.get() as usize;
            segment.size = raw.size.get() as usize;
        }

        // The writable segment is the one the zero-filled tail extends, so it
        // is mapped long enough to hold both. Every other segment is mapped at
        // the size the header gives it.
        segments[2].size = page_align_up(segments[2].size + bss_size);
        let writable = segments[2];

        Self {
            segments,
            mapped_size: page_align_up(header.size.get() as usize + bss_size),
            image_size: writable.file_off + writable.size,
        }
    }
}

/// One segment, as an offset into the image and a length.
#[derive(Debug, Clone, Copy)]
struct Segment {
    file_off: usize,
    size: usize,
}

/// Rounds `size` up to the next page boundary.
fn page_align_up(size: usize) -> usize {
    size.next_multiple_of(PAGE_SIZE)
}

/// Error returned by [`MappedNro::unmap`].
///
/// The kernel refused to remove a segment's mapping. Segments before it are
/// already unmapped, so the program is left partly removed and the range it
/// occupied is not reusable as it was.
#[derive(Debug, thiserror::Error)]
#[error("the kernel refused to unmap a segment")]
pub struct UnmapError(#[source] pub nx_svc::mem::UnmapProcessCodeMemoryError);
