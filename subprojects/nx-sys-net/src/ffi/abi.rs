//! The C types the socket calls exchange.
//!
//! Every layout here is pinned against the BSD headers this workspace's C code compiles against,
//! under `subprojects/libnx/src/nx/external/bsd/include/`. Getting one wrong corrupts a caller's
//! stack rather than failing a test, so each field is transcribed rather than inferred, and the
//! order is the header's order.
//!
//! Nothing here interprets an address. A `sockaddr` reaches the service as the bytes the caller
//! supplied, exactly as the C driver passes them, because the service is what decides which
//! families it accepts. What this module contributes is the length: a C caller passes a pointer
//! and a `socklen_t` as two arguments, and turning that pair into one bounded slice before
//! anything reads it is what [`sockaddr_bytes`] exists for.

use core::ffi::{
    c_int,
    c_uint,
    c_void,
};

/// Length of a socket address, as C declares it.
pub type SockLenT = u32;

/// Signed byte count, as C declares it.
pub type SsizeT = isize;

/// Descriptor count for `poll`, as C declares it.
pub type NfdsT = c_uint;

/// A scatter-gather element.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IoVec {
    /// Start of the region.
    pub iov_base: *mut c_void,
    /// Length of the region, in bytes.
    pub iov_len: usize,
}

/// A message header, as `sendmsg` and `recvmsg` take it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MsgHdr {
    /// Optional address; null when the socket is connected.
    pub msg_name: *mut c_void,
    /// Length of the address at `msg_name`.
    pub msg_namelen: SockLenT,
    /// The scatter-gather array.
    pub msg_iov: *mut IoVec,
    /// How many elements `msg_iov` has.
    pub msg_iovlen: c_int,
    /// Ancillary data.
    pub msg_control: *mut c_void,
    /// Length of the ancillary data.
    pub msg_controllen: SockLenT,
    /// Flags on the received message.
    pub msg_flags: c_int,
}

/// One entry of the array the multi-message calls take.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MMsgHdr {
    /// The message itself.
    pub msg_hdr: MsgHdr,
    /// How many bytes the message carried, written back by the call.
    pub msg_len: SsizeT,
}

/// Ancillary-data header, read only far enough to refuse what the service will not carry.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CMsgHdr {
    /// Length of this ancillary element, including the header.
    pub cmsg_len: SockLenT,
    /// Originating protocol.
    pub cmsg_level: c_int,
    /// Protocol-specific type.
    pub cmsg_type: c_int,
}

/// One descriptor's readiness request and result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PollFd {
    /// The descriptor to watch. A negative value asks for nothing and reports nothing.
    pub fd: c_int,
    /// What to watch for.
    pub events: i16,
    /// What happened, written by the call.
    pub revents: i16,
}

/// A duration in seconds and microseconds.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TimeVal {
    /// Whole seconds.
    pub tv_sec: i64,
    /// Microseconds past `tv_sec`.
    pub tv_usec: i64,
}

/// A duration in seconds and nanoseconds.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TimeSpec {
    /// Whole seconds.
    pub tv_sec: i64,
    /// Nanoseconds past `tv_sec`.
    pub tv_nsec: i64,
}

/// How many descriptors an `fd_set` tracks.
///
/// Fixed by the C headers, which size the caller's `fd_set` with it.
pub const FD_SETSIZE: usize = 64;

/// The word an `fd_set` is an array of.
pub type FdMask = u64;

/// How many bits one [`FdMask`] holds.
pub const NFDBITS: usize = FdMask::BITS as usize;

/// A descriptor set, as `select` takes it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FdSet {
    /// One bit per descriptor, least significant bit first.
    pub fds_bits: [FdMask; FD_SETSIZE.div_ceil(NFDBITS)],
}

impl FdSet {
    /// Whether `fd` is in the set.
    ///
    /// A descriptor outside the set's range is not in it, rather than an out-of-bounds read.
    pub fn contains(&self, fd: usize) -> bool {
        match self.fds_bits.get(fd / NFDBITS) {
            Some(word) => word & (1 << (fd % NFDBITS)) != 0,
            None => false,
        }
    }

    /// Adds `fd` to the set, ignoring a descriptor outside its range.
    pub fn insert(&mut self, fd: usize) {
        if let Some(word) = self.fds_bits.get_mut(fd / NFDBITS) {
            *word |= 1 << (fd % NFDBITS);
        }
    }

    /// Empties the set.
    pub fn clear(&mut self) {
        self.fds_bits = [0; FD_SETSIZE.div_ceil(NFDBITS)];
    }
}

/// Borrows a caller's socket address as bytes.
///
/// A C caller passes the address as a pointer and a length that are separate arguments and can
/// disagree. This is where they stop being separate: what comes out is one slice or nothing, and
/// nothing downstream can read past the length the caller declared.
///
/// Returns `None` for a null pointer or a zero length, which together are how C says "no address".
///
/// # Safety
///
/// `addr` must be null, or point to at least `len` readable bytes that stay valid and unwritten
/// for the lifetime of the returned slice.
pub unsafe fn sockaddr_bytes<'a>(addr: *const c_void, len: SockLenT) -> Option<&'a [u8]> {
    if addr.is_null() || len == 0 {
        return None;
    }

    // SAFETY: the caller guarantees `len` readable bytes at `addr` for the returned lifetime.
    Some(unsafe { core::slice::from_raw_parts(addr.cast::<u8>(), len as usize) })
}

/// Borrows a caller's address as the owned form the commands take.
///
/// Returns `None` when the caller supplied no address, and when it supplied one longer than any
/// address family the service supports.
///
/// # Safety
///
/// `addr` must be null, or point to at least `len` readable bytes.
pub unsafe fn borrow_sockaddr(
    addr: *const c_void,
    len: SockLenT,
) -> Option<nx_service_bsd::RawSockAddr> {
    // SAFETY: the caller guarantees `len` readable bytes at `addr`.
    let bytes = unsafe { sockaddr_bytes(addr, len) }?;
    nx_service_bsd::RawSockAddr::try_from(bytes).ok()
}

/// Writes an address the service reported back into a caller's buffer and length.
///
/// Follows the C convention exactly: at most as many bytes as the buffer holds are copied, and the
/// length written back is the address's *real* length, which may exceed the buffer. That is how C
/// reports truncation, and a caller that reads the length as "bytes written" is the reason
/// [`nx_service_bsd::RawSockAddr`] refuses to hand the pair around internally.
///
/// Does nothing when either pointer is null, which is how a caller says it does not want the
/// address.
///
/// # Safety
///
/// `addr` must be null or point to at least `*addr_len` writable bytes, and `addr_len` must be
/// null or point to a writable [`SockLenT`].
pub unsafe fn write_sockaddr(
    addr: *mut c_void,
    addr_len: *mut SockLenT,
    reported: &nx_service_bsd::RawSockAddr,
) {
    if addr.is_null() || addr_len.is_null() {
        return;
    }

    // SAFETY: the caller guarantees `addr_len` points to a writable length.
    let capacity = unsafe { *addr_len } as usize;
    let bytes = reported.as_bytes();
    let copied = core::cmp::min(capacity, bytes.len());

    // SAFETY: `copied` is bounded by the caller's declared capacity and by the source length.
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr.cast::<u8>(), copied) };
    // SAFETY: as above; this reports the real length, which is what C callers expect.
    // Bounded by `RawSockAddr::CAPACITY`, which is far below what a `SockLenT` holds.
    unsafe { *addr_len = bytes.len() as SockLenT };
}
