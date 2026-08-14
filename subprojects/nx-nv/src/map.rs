//! Memory objects on the NV driver's memory-object device.
//!
//! A memory object is a buffer this process allocated and then handed to the
//! driver, which pins its pages and gives back two names for it: a handle,
//! valid only here, and an id, valid in any process that is told it. Handing
//! the id to a compositor is how a buffer this process drew into becomes a
//! buffer something else can display.
//!
//! ## Two names, two lifetimes
//!
//! The handle is a reference the driver counts. Releasing it is this process
//! saying "I am done", not "tear this down": an object whose id another
//! process resolved outlives the release, which is the whole point of the id.
//! So [`MemoryMap`] owns the handle and nothing else — it is neither the
//! buffer's allocator nor the object's sole keeper.
//!
//! Because the handle is counted and the release is a request, releasing twice
//! is not caught by anything: the second release decrements a count that has
//! since been raised by an unrelated allocation, and that object is torn down
//! early instead. That is why the handle is owned by a type that cannot be
//! copied rather than carried as a number beside a `free` call, and why
//! [`MapHandle`] — the number — closes nothing.
//!
//! ## Uncached objects change the caller's mapping
//!
//! A device that reads memory without going through the CPU's caches needs the
//! buffer's lines written back first, and needs the CPU to stop caching it
//! afterwards. That second part changes the *caller's* mapping, not the
//! driver's, so it has to be undone when the object is released — otherwise a
//! buffer handed back to the allocator stays uncached forever and every later
//! user of those pages pays for it.

mod ioctl;

use core::{
    mem::ManuallyDrop,
    ptr::NonNull,
};

use nx_service_nv::{
    IoctlError,
    NvService,
    fd::Fd,
};
use zerocopy::{
    FromBytes,
    IntoBytes,
};

/// The device path memory objects are allocated on.
const DEVICE_PATH: &str = "/dev/nvmap";

/// The page size the device measures buffers and alignments in.
pub const PAGE_SIZE: usize = 0x1000;

/// The memory-attribute bit that marks a range uncached.
const ATTR_UNCACHED: u32 = 8;

/// An open connection to the memory-object device.
///
/// The driver session it borrows belongs to whoever established it: a process
/// gets one, and opening a second would not get a second.
pub struct NvMapDevice<'s> {
    service: &'s NvService,
    fd: Fd,
}

impl<'s> NvMapDevice<'s> {
    /// Opens the memory-object device over an established driver session.
    ///
    /// # Errors
    ///
    /// Returns [`OpenError`] when the driver refuses the device, which happens
    /// when the session was established for a service that does not carry it.
    pub fn open(service: &'s NvService) -> Result<Self, OpenError> {
        let fd = service.open(DEVICE_PATH).map_err(OpenError)?;
        Ok(Self { service, fd })
    }

    /// Adopts a device descriptor opened over `service`.
    ///
    /// The caller must ensure `fd` names an open memory-object device on that
    /// session and that nothing else will close it, since this value closes it
    /// on drop. A second owner sends its close against a descriptor the driver
    /// may have reissued, closing an unrelated device rather than faulting,
    /// which is why this is a safe function.
    #[inline]
    pub const fn from_raw_unchecked(service: &'s NvService, fd: Fd) -> Self {
        Self { service, fd }
    }

    /// Returns the descriptor the device is open on.
    #[inline]
    pub fn fd(&self) -> Fd {
        self.fd
    }

    /// Hands the close obligation back to the caller.
    ///
    /// The device stays open: whoever holds the returned descriptor now owes
    /// the driver exactly one close, and [`NvMapDevice::from_raw_unchecked`]
    /// is how to give it back.
    pub fn into_fd(self) -> Fd {
        let this = ManuallyDrop::new(self);
        this.fd
    }

    /// Borrows the device for the length of one operation.
    #[inline]
    pub fn as_borrowed(&self) -> BorrowedMapDevice<'_> {
        BorrowedMapDevice {
            service: self.service,
            fd: self.fd,
        }
    }

    /// Allocates a memory object over `buffer` and pins its pages.
    ///
    /// See [`BorrowedMapDevice::create_map`].
    ///
    /// # Errors
    ///
    /// Returns [`CreateMapError`] when any step is refused.
    pub fn create_map(
        &self,
        buffer: MapBuffer,
        align: MapAlign,
        kind: MapKind,
        cacheable: bool,
    ) -> Result<MemoryMap<'_>, CreateMapError> {
        self.as_borrowed()
            .create_map(buffer, align, kind, cacheable)
    }

    /// Adopts the memory object `id` names, allocated by another process.
    ///
    /// See [`BorrowedMapDevice::adopt_map`].
    ///
    /// # Errors
    ///
    /// Returns [`AdoptMapError`] when the id names no live object.
    pub fn adopt_map(&self, id: MapId) -> Result<MemoryMap<'_>, AdoptMapError> {
        self.as_borrowed().adopt_map(id)
    }
}

impl Drop for NvMapDevice<'_> {
    fn drop(&mut self) {
        // A refused close leaves one descriptor behind for the life of the
        // process. Nothing useful can be done about it from a destructor, and
        // the alternative — not closing — leaks it just the same.
        let _ = self.service.close_fd(self.fd);
    }
}

/// Errors returned by [`NvMapDevice::open`].
///
/// The driver refused to open the memory-object device.
#[derive(Debug, thiserror::Error)]
#[error("Failed to open the memory-object device")]
pub struct OpenError(#[source] pub nx_service_nv::OpenError);

/// An open memory-object device borrowed for the length of an operation.
///
/// Closes nothing: [`NvMapDevice`] owns the descriptor. This is the form every
/// operation takes, so that a caller holding the descriptor somewhere the
/// borrow checker cannot see it — a C caller's global, say — can still reach
/// them without a value whose destructor would close a descriptor it does not
/// own.
#[derive(Clone, Copy)]
pub struct BorrowedMapDevice<'d> {
    service: &'d NvService,
    fd: Fd,
}

impl<'d> BorrowedMapDevice<'d> {
    /// Borrows a device descriptor opened elsewhere.
    ///
    /// The caller must ensure `fd` names this session's memory-object device
    /// and is still open. An unknown descriptor is answered with a driver
    /// error rather than faulting, which is why this is a safe function.
    #[inline]
    pub const fn from_raw_unchecked(service: &'d NvService, fd: Fd) -> Self {
        Self { service, fd }
    }

    /// Returns the descriptor the device is open on.
    #[inline]
    pub const fn fd(&self) -> Fd {
        self.fd
    }

    /// Allocates a memory object over `buffer` and pins its pages.
    ///
    /// `cacheable` says whether the CPU may keep the buffer in its caches. A
    /// buffer read by a device that does not see those caches must pass
    /// `false`, which writes the buffer back and marks the range uncached for
    /// as long as the object lives.
    ///
    /// # Errors
    ///
    /// Returns [`CreateMapError`] when any step is refused. The object is
    /// released before returning, so a failure leaves nothing allocated and
    /// the buffer's mapping unchanged.
    pub fn create_map(
        self,
        buffer: MapBuffer,
        align: MapAlign,
        kind: MapKind,
        cacheable: bool,
    ) -> Result<MemoryMap<'d>, CreateMapError> {
        let mut create = ioctl::Create {
            size: buffer.len(),
            handle: 0,
        };
        self.request(ioctl::CREATE, &mut create)
            .map_err(CreateMapError::Create)?;

        // The object exists on the driver side from here on, so it is wrapped
        // in its owner immediately: every `?` below then releases it on the
        // way out instead of leaving it pinned.
        let mut map = MemoryMap {
            device: self,
            handle: MapHandle(create.handle),
            id: MapId(0),
            buffer: Some(buffer),
            cacheable,
        };

        let mut alloc = ioctl::Alloc {
            handle: create.handle,
            heapmask: 0,
            flags: u32::from(cacheable),
            align: align.to_raw(),
            kind: kind.to_raw(),
            pad: [0; 7],
            addr: buffer.as_ptr().as_ptr() as u64,
        };
        self.request(ioctl::ALLOC, &mut alloc)
            .map_err(CreateMapError::Alloc)?;

        if !cacheable {
            buffer
                .set_uncached(true)
                .map_err(CreateMapError::MarkUncached)?;
        }

        let mut get_id = ioctl::GetId {
            id: 0,
            handle: create.handle,
        };
        self.request(ioctl::GET_ID, &mut get_id)
            .map_err(CreateMapError::GetId)?;

        map.id = MapId(get_id.id);
        Ok(map)
    }

    /// Adopts the memory object `id` names, allocated by another process.
    ///
    /// The returned object has no buffer: the pages belong to whoever
    /// allocated them, and this process holds only a counted reference.
    ///
    /// # Errors
    ///
    /// Returns [`AdoptMapError`] when the id names no live object, or when its
    /// properties cannot be read.
    pub fn adopt_map(self, id: MapId) -> Result<MemoryMap<'d>, AdoptMapError> {
        let mut from_id = ioctl::FromId {
            id: id.to_raw(),
            handle: 0,
        };
        self.request(ioctl::FROM_ID, &mut from_id)
            .map_err(AdoptMapError)?;

        Ok(MemoryMap {
            device: self,
            handle: MapHandle(from_id.handle),
            id,
            buffer: None,
            cacheable: true,
        })
    }

    /// Reads one property of a memory object.
    ///
    /// # Errors
    ///
    /// Returns the driver's error when the property or the handle is refused.
    fn read_param(self, handle: MapHandle, param: u32) -> Result<u32, IoctlError> {
        let mut request = ioctl::Param {
            handle: handle.to_raw(),
            param,
            result: 0,
        };
        self.request(ioctl::PARAM, &mut request)?;
        Ok(request.result)
    }

    /// Sends one request, letting the driver update the payload in place.
    fn request<T>(self, code: u32, payload: &mut T) -> Result<(), IoctlError>
    where
        T: FromBytes + IntoBytes,
    {
        self.service.ioctl(self.fd, code, payload.as_mut_bytes())
    }
}

/// A memory object this process holds a counted reference to.
///
/// Releasing the reference is what dropping this does; it is not `Copy` or
/// `Clone` because a second value would release a second time.
pub struct MemoryMap<'d> {
    device: BorrowedMapDevice<'d>,
    handle: MapHandle,
    id: MapId,
    /// The buffer backing the object, absent when it was adopted from another
    /// process and these pages are not ours.
    buffer: Option<MapBuffer>,
    cacheable: bool,
}

impl<'d> MemoryMap<'d> {
    /// Adopts an object this process already holds a reference to.
    ///
    /// The caller must ensure `handle` names a live object whose reference
    /// nothing else will release, since this value releases it on drop. A
    /// second owner sends its release against a count an unrelated allocation
    /// has since raised, tearing that object down rather than faulting, which
    /// is why this is a safe function.
    ///
    /// This exists for the C-facing surface, which stores the object's fields
    /// in a caller-owned struct and rebuilds the owner when the caller asks
    /// for the release.
    pub fn from_raw_unchecked(
        device: BorrowedMapDevice<'d>,
        handle: MapHandle,
        id: MapId,
        buffer: Option<MapBuffer>,
        cacheable: bool,
    ) -> Self {
        Self {
            device,
            handle,
            id,
            buffer,
            cacheable,
        }
    }

    /// Returns the handle naming the object in this process.
    #[inline]
    pub fn handle(&self) -> MapHandle {
        self.handle
    }

    /// Hands the release obligation back to the caller.
    ///
    /// The object stays referenced and the buffer's mapping is left as it is:
    /// whoever holds the returned handle now owes the driver exactly one
    /// release, and [`MemoryMap::from_raw_unchecked`] is how to give it back.
    ///
    /// This is for callers that keep the object's fields somewhere the borrow
    /// checker cannot follow — a C caller's struct, say — and ask for the
    /// release later.
    pub fn into_handle(self) -> MapHandle {
        let this = ManuallyDrop::new(self);
        this.handle
    }

    /// Returns the id naming the object in any process that is told it.
    #[inline]
    pub fn id(&self) -> MapId {
        self.id
    }

    /// Returns the buffer backing the object, if these pages are ours.
    #[inline]
    pub fn buffer(&self) -> Option<MapBuffer> {
        self.buffer
    }

    /// Returns whether the CPU may keep the buffer in its caches.
    #[inline]
    pub fn is_cacheable(&self) -> bool {
        self.cacheable
    }

    /// Reads the object's size from the driver.
    ///
    /// # Errors
    ///
    /// Returns the driver's error when the property cannot be read.
    pub fn size(&self) -> Result<u32, IoctlError> {
        self.device.read_param(self.handle, ioctl::param::SIZE)
    }

    /// Reads the memory layout the GPU reads the object with.
    ///
    /// # Errors
    ///
    /// Returns the driver's error when the property cannot be read.
    pub fn kind(&self) -> Result<MapKind, IoctlError> {
        let raw = self.device.read_param(self.handle, ioctl::param::KIND)?;
        Ok(MapKind::from_raw(raw as u8))
    }
}

impl Drop for MemoryMap<'_> {
    fn drop(&mut self) {
        if !self.cacheable
            && let Some(buffer) = self.buffer
        {
            // The pages go back to the allocator when the caller is done with
            // them, so the mapping has to be restored even if the driver
            // refuses the release below. A failure here cannot be reported
            // from a destructor and cannot be retried.
            let _ = buffer.set_uncached(false);
        }

        let mut free = ioctl::Free {
            handle: self.handle.to_raw(),
            ..Default::default()
        };
        // A refused release leaves the reference standing for the life of the
        // process; there is no second way to drop it.
        let _ = self.device.request(ioctl::FREE, &mut free);
    }
}

/// Errors returned by [`NvMapDevice::create_map`].
#[derive(Debug, thiserror::Error)]
pub enum CreateMapError {
    /// The driver refused to allocate the object
    #[error("Failed to allocate the memory object")]
    Create(#[source] IoctlError),

    /// The driver refused to pin the buffer's pages
    #[error("Failed to bind the buffer to the memory object")]
    Alloc(#[source] IoctlError),

    /// The buffer's mapping could not be marked uncached
    #[error("Failed to mark the buffer uncached")]
    MarkUncached(#[source] CacheAttrError),

    /// The driver refused to report the object's shareable id
    #[error("Failed to read the memory object's id")]
    GetId(#[source] IoctlError),
}

/// Errors returned by [`NvMapDevice::adopt_map`].
///
/// The id names no object this process may reference.
#[derive(Debug, thiserror::Error)]
#[error("Failed to resolve the memory object id")]
pub struct AdoptMapError(#[source] pub IoctlError);

/// Names a memory object inside this process.
///
/// A number that releases nothing: [`MemoryMap`] owns the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MapHandle(u32);

impl MapHandle {
    /// Wraps a handle the driver issued.
    ///
    /// The caller must ensure `raw` came from this device and has not been
    /// released. The driver owns the table, so nothing here can check it; an
    /// unknown handle is answered with an error rather than faulting, which is
    /// why this is a safe function.
    #[inline]
    pub const fn from_raw_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw handle the driver issued.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

/// Names a memory object in any process that is told it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MapId(u32);

impl MapId {
    /// Wraps an id the driver issued, or that another process passed along.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw id.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

/// The memory layout the GPU reads a memory object with.
///
/// The driver accepts a wide range of tiled layouts; the ones named here are
/// those this workspace allocates today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MapKind(u8);

impl MapKind {
    /// Linear memory, addressed row by row.
    pub const PITCH: Self = Self(0x00);

    /// The 16-bytes-by-2 block layout used for colour surfaces.
    pub const GENERIC_16BX2: Self = Self(0xFE);

    /// Wraps a layout code the driver defines.
    #[inline]
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    /// Returns the raw layout code.
    #[inline]
    pub const fn to_raw(self) -> u8 {
        self.0
    }
}

/// The alignment a memory object's pages are placed at.
///
/// The driver requires a power of two no smaller than a page, so the value is
/// checked once here rather than at every call that passes one along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MapAlign(u32);

impl MapAlign {
    /// A single page, the smallest alignment the driver accepts.
    pub const PAGE: Self = Self(PAGE_SIZE as u32);

    /// Returns the raw alignment in bytes.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for MapAlign {
    type Error = MapAlignError;

    fn try_from(bytes: u32) -> Result<Self, Self::Error> {
        if bytes < PAGE_SIZE as u32 {
            return Err(MapAlignError::BelowPage { bytes });
        }
        if !bytes.is_power_of_two() {
            return Err(MapAlignError::NotPowerOfTwo { bytes });
        }
        Ok(Self(bytes))
    }
}

/// Errors returned when building a [`MapAlign`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MapAlignError {
    /// The alignment is smaller than a page
    #[error("Alignment {bytes:#x} is smaller than a page")]
    BelowPage {
        /// The rejected alignment.
        bytes: u32,
    },

    /// The alignment is not a power of two
    #[error("Alignment {bytes:#x} is not a power of two")]
    NotPowerOfTwo {
        /// The rejected alignment.
        bytes: u32,
    },
}

/// A CPU buffer that can back a memory object.
///
/// The driver pins whole pages, so the address and the length are both checked
/// against the page size once, here, rather than being taken on trust at the
/// request that would fail on them.
///
/// This names memory the caller allocated; it does not own it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapBuffer {
    ptr: NonNull<u8>,
    len: u32,
}

impl MapBuffer {
    /// Checks that `ptr` and `len` describe a range the driver can pin.
    ///
    /// # Errors
    ///
    /// Returns [`MapBufferError`] when the address is not page-aligned, or the
    /// length is zero, not a multiple of the page size, or too large for the
    /// driver's 32-bit size field.
    pub fn create(ptr: NonNull<u8>, len: usize) -> Result<Self, MapBufferError> {
        if len == 0 {
            return Err(MapBufferError::Empty);
        }
        if !len.is_multiple_of(PAGE_SIZE) {
            return Err(MapBufferError::LengthNotPaged { len });
        }
        let Ok(len) = u32::try_from(len) else {
            return Err(MapBufferError::LengthTooLarge { len });
        };

        let addr = ptr.as_ptr() as usize;
        if !addr.is_multiple_of(PAGE_SIZE) {
            return Err(MapBufferError::AddressNotPaged { addr });
        }

        Ok(Self { ptr, len })
    }

    /// Returns the start of the buffer.
    #[inline]
    pub const fn as_ptr(self) -> NonNull<u8> {
        self.ptr
    }

    /// Returns the buffer's length in bytes.
    #[inline]
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Returns whether the buffer is empty, which it never is.
    ///
    /// A [`MapBuffer`] cannot be built over an empty range; this exists so
    /// callers reading `len()` are not steered toward a length comparison.
    #[inline]
    pub const fn is_empty(self) -> bool {
        false
    }

    /// Sets or clears the uncached attribute on the buffer's mapping.
    ///
    /// Turning it on writes the buffer back first, so that a device reading
    /// main memory sees what the CPU last wrote rather than what was there
    /// before.
    ///
    /// # Errors
    ///
    /// Returns [`CacheAttrError`] when the kernel refuses the change.
    fn set_uncached(self, uncached: bool) -> Result<(), CacheAttrError> {
        if uncached {
            // SAFETY: `MapBuffer` was built over a mapped, page-aligned range
            // of `len` bytes, which is what the maintenance instructions need.
            unsafe { nx_cpu::cache::flush_data_range(self.ptr.as_ptr(), self.len as usize) };
        }

        let attr = if uncached { ATTR_UNCACHED } else { 0 };
        // SAFETY: the range is page-aligned and page-sized by construction and
        // belongs to this process, which is what the call requires.
        let rc = unsafe {
            nx_svc::raw::set_memory_attribute(
                self.ptr.as_ptr().cast(),
                self.len as usize,
                ATTR_UNCACHED,
                attr,
            )
        };
        if rc != 0 {
            return Err(CacheAttrError { rc });
        }
        Ok(())
    }
}

// SAFETY: a `MapBuffer` is an address and a length. It confers no access of
// its own, so moving one between threads cannot race; whatever the pointer
// names is governed by the rules of that memory, not of this handle.
unsafe impl Send for MapBuffer {}

// SAFETY: see the `Send` impl — sharing a reference hands out the same address
// and length, which is not access.
unsafe impl Sync for MapBuffer {}

/// Errors returned by [`MapBuffer::create`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MapBufferError {
    /// The range is empty
    #[error("A memory object cannot be backed by an empty buffer")]
    Empty,

    /// The length is not a whole number of pages
    #[error("Length {len:#x} is not a multiple of the page size")]
    LengthNotPaged {
        /// The rejected length.
        len: usize,
    },

    /// The length does not fit the driver's size field
    #[error("Length {len:#x} does not fit a 32-bit size")]
    LengthTooLarge {
        /// The rejected length.
        len: usize,
    },

    /// The start address is not page-aligned
    #[error("Address {addr:#x} is not page-aligned")]
    AddressNotPaged {
        /// The rejected address.
        addr: usize,
    },
}

/// Error returned when changing a buffer's cache attribute.
///
/// The kernel refused the attribute change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("The kernel refused the cache attribute change ({rc:#x})")]
pub struct CacheAttrError {
    /// The result code the kernel returned.
    pub rc: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page-aligned address that is never dereferenced.
    fn paged_ptr() -> NonNull<u8> {
        NonNull::new(PAGE_SIZE as *mut u8).expect("a page address is never null")
    }

    #[test]
    fn buffer_accepts_a_page_aligned_range() {
        //* Given
        let len = PAGE_SIZE * 4;

        //* When
        let result = MapBuffer::create(paged_ptr(), len);

        //* Then
        let buffer = result.expect("a page-aligned range should be accepted");
        assert_eq!(
            buffer.len(),
            len as u32,
            "the length must survive the check"
        );
    }

    #[test]
    fn buffer_rejects_a_length_that_is_not_a_whole_page() {
        //* Given
        let len = PAGE_SIZE + 1;

        //* When
        let result = MapBuffer::create(paged_ptr(), len);

        //* Then
        assert_eq!(
            result,
            Err(MapBufferError::LengthNotPaged { len }),
            "a partial page must be rejected before the driver sees it"
        );
    }

    #[test]
    fn buffer_rejects_an_unaligned_address() {
        //* Given
        let addr = PAGE_SIZE + 8;
        let ptr = NonNull::new(addr as *mut u8).expect("a non-zero address is never null");

        //* When
        let result = MapBuffer::create(ptr, PAGE_SIZE);

        //* Then
        assert_eq!(
            result,
            Err(MapBufferError::AddressNotPaged { addr }),
            "an unaligned start must be rejected"
        );
    }

    #[test]
    fn buffer_rejects_an_empty_range() {
        //* Given / When
        let result = MapBuffer::create(paged_ptr(), 0);

        //* Then
        assert_eq!(
            result,
            Err(MapBufferError::Empty),
            "zero pages is not a buffer"
        );
    }

    #[test]
    fn align_accepts_a_page_multiple_power_of_two() {
        //* Given
        let bytes = 0x20000_u32;

        //* When
        let result = MapAlign::try_from(bytes);

        //* Then
        let align = result.expect("a page-multiple power of two should be accepted");
        assert_eq!(
            align.to_raw(),
            bytes,
            "the alignment must survive the check"
        );
    }

    #[test]
    fn align_rejects_a_value_below_one_page() {
        //* Given
        let bytes = 0x800_u32;

        //* When
        let result = MapAlign::try_from(bytes);

        //* Then
        assert_eq!(
            result,
            Err(MapAlignError::BelowPage { bytes }),
            "the driver pins whole pages, so a smaller alignment is meaningless"
        );
    }

    #[test]
    fn align_rejects_a_non_power_of_two() {
        //* Given
        let bytes = 0x3000_u32;

        //* When
        let result = MapAlign::try_from(bytes);

        //* Then
        assert_eq!(
            result,
            Err(MapAlignError::NotPowerOfTwo { bytes }),
            "the driver rounds by masking, which needs a power of two"
        );
    }
}
