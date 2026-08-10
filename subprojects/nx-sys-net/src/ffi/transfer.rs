//! Moving data through a socket.
//!
//! The simple calls — `send`, `recv` and the address-carrying variants — hand the service a single
//! buffer and are little more than a descriptor lookup and a flag conversion.
//!
//! The scatter-gather calls are not simple, and the reason is that the service has no
//! scatter-gather command. `sendmsg` gives a caller an array of `iovec`s pointing anywhere in its
//! address space; the service takes one flat buffer. So the whole message vector — addresses,
//! lengths, payloads and ancillary data — is packed into a single contiguous request, sent, and
//! unpacked back into the caller's structures on return. [`pack`] and [`unpack`] are the two
//! halves, and the layout they agree on is the service's, not this crate's.
//!
//! ## The layout
//!
//! One leading byte, then per message: the address length and the address, the `iovec` count, then
//! each element's length followed by its bytes, then the ancillary length and its bytes, then the
//! message flags and the message length. Every scalar is written in native byte order, because the
//! service runs on the same machine.
//!
//! ## Why unpacking checks every step
//!
//! The buffer coming back was written by the service, and the lengths in it decide how far this
//! code reads and how much it copies into a caller's buffers. A length larger than the caller
//! declared, or one that walks past the end of the response, is a write past the end of somebody's
//! allocation. So each is checked against both bounds before it is used, and a failure abandons the
//! whole unpack rather than proceeding with the entries it had already accepted.

use alloc::{
    vec,
    vec::Vec,
};
use core::ffi::{
    c_int,
    c_uint,
    c_void,
};

use nx_service_bsd::{
    RecvFlags,
    RecvTimeout,
    SendFlags,
};

use super::{
    abi::{
        CMsgHdr,
        MMsgHdr,
        MsgHdr,
        SockLenT,
        SsizeT,
        TimeSpec,
        borrow_sockaddr,
        write_sockaddr,
    },
    descriptor::with_socket,
    errno,
};

/// Receives from a connected socket.
///
/// # Safety
///
/// `buf` must point to at least `len` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__recv(
    sockfd: c_int,
    buf: *mut c_void,
    len: usize,
    flags: c_int,
) -> SsizeT {
    // SAFETY: the caller guarantees `len` writable bytes at `buf`.
    let Some(buf) = (unsafe { as_mut_slice(buf, len) }) else {
        return to_failure(errno::fail(errno::EFAULT));
    };
    let flags = RecvFlags::from_bits_truncate(flags);

    match with_socket(sockfd, |svc, sock| svc.recv(sock, buf, flags)) {
        Ok(count) => to_ssize(count),
        Err(failure) => to_failure(failure),
    }
}

/// Receives, and reports the sender's address.
///
/// # Safety
///
/// `buf` must point to at least `len` writable bytes; `src_addr` must be null or point to at least
/// `*addr_len` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__recvfrom(
    sockfd: c_int,
    buf: *mut c_void,
    len: usize,
    flags: c_int,
    src_addr: *mut c_void,
    addr_len: *mut SockLenT,
) -> SsizeT {
    // SAFETY: the caller guarantees `len` writable bytes at `buf`.
    let Some(buf) = (unsafe { as_mut_slice(buf, len) }) else {
        return to_failure(errno::fail(errno::EFAULT));
    };
    let flags = RecvFlags::from_bits_truncate(flags);

    match with_socket(sockfd, |svc, sock| svc.recv_from(sock, buf, flags)) {
        Ok((count, from)) => {
            // SAFETY: the caller guarantees the address buffer and length pointers.
            unsafe { write_sockaddr(src_addr, addr_len, &from) };
            to_ssize(count)
        }
        Err(failure) => to_failure(failure),
    }
}

/// Sends on a connected socket.
///
/// # Safety
///
/// `buf` must point to at least `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__send(
    sockfd: c_int,
    buf: *const c_void,
    len: usize,
    flags: c_int,
) -> SsizeT {
    // SAFETY: the caller guarantees `len` readable bytes at `buf`.
    let Some(buf) = (unsafe { as_slice(buf, len) }) else {
        return to_failure(errno::fail(errno::EFAULT));
    };
    let flags = SendFlags::from_bits_truncate(flags);

    match with_socket(sockfd, |svc, sock| svc.send(sock, buf, flags)) {
        Ok(count) => to_ssize(count),
        Err(failure) => to_failure(failure),
    }
}

/// Sends to an explicit address.
///
/// # Safety
///
/// `buf` must point to at least `len` readable bytes; `dest_addr` must be null or point to at
/// least `addr_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sendto(
    sockfd: c_int,
    buf: *const c_void,
    len: usize,
    flags: c_int,
    dest_addr: *const c_void,
    addr_len: SockLenT,
) -> SsizeT {
    // SAFETY: the caller guarantees `len` readable bytes at `buf`.
    let Some(buf) = (unsafe { as_slice(buf, len) }) else {
        return to_failure(errno::fail(errno::EFAULT));
    };
    let flags = SendFlags::from_bits_truncate(flags);

    // A null destination makes this an ordinary send, which is what C specifies.
    // SAFETY: the caller guarantees `addr_len` readable bytes at `dest_addr`.
    let Some(dest) = (unsafe { borrow_sockaddr(dest_addr, addr_len) }) else {
        return match with_socket(sockfd, |svc, sock| svc.send(sock, buf, flags)) {
            Ok(count) => to_ssize(count),
            Err(failure) => to_failure(failure),
        };
    };

    match with_socket(sockfd, |svc, sock| svc.send_to(sock, buf, flags, &dest)) {
        Ok(count) => to_ssize(count),
        Err(failure) => to_failure(failure),
    }
}

/// Receives one message with scatter-gather buffers and ancillary data.
///
/// # Safety
///
/// `msg` must be null or point to a writable [`MsgHdr`] whose buffers are valid as it describes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__recvmsg(
    sockfd: c_int,
    msg: *mut MsgHdr,
    flags: c_int,
) -> SsizeT {
    if msg.is_null() {
        return to_failure(errno::fail(errno::EINVAL));
    }

    // SAFETY: the caller guarantees a writable header at a non-null pointer.
    let mut vec = [MMsgHdr {
        msg_hdr: unsafe { *msg },
        msg_len: 0,
    }];

    // SAFETY: the single entry borrows the caller's own header, whose buffers the caller vouched
    // for.
    let ret = unsafe { recv_mmsg_inner(sockfd, vec.as_mut_ptr(), 1, flags, core::ptr::null()) };
    if ret < 0 {
        return to_failure(ret);
    }

    // SAFETY: as above; the header is written back with the lengths the service reported.
    unsafe { *msg = vec[0].msg_hdr };
    vec[0].msg_len
}

/// Sends one message with scatter-gather buffers and ancillary data.
///
/// # Safety
///
/// `msg` must be null or point to a readable [`MsgHdr`] whose buffers are valid as it describes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sendmsg(
    sockfd: c_int,
    msg: *const MsgHdr,
    flags: c_int,
) -> SsizeT {
    if msg.is_null() {
        return to_failure(errno::fail(errno::EINVAL));
    }

    // SAFETY: the caller guarantees a readable header at a non-null pointer.
    let mut vec = [MMsgHdr {
        msg_hdr: unsafe { *msg },
        msg_len: 0,
    }];

    // SAFETY: the single entry borrows the caller's own header, whose buffers the caller vouched
    // for.
    let ret = unsafe { send_mmsg_inner(sockfd, vec.as_mut_ptr(), 1, flags) };
    if ret <= 0 {
        return to_failure(ret);
    }

    vec[0].msg_len
}

/// Sends up to `vlen` messages in one request.
///
/// # Safety
///
/// `msgvec` must point to `vlen` readable and writable [`MMsgHdr`]s whose buffers are valid as
/// they describe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sendmmsg(
    sockfd: c_int,
    msgvec: *mut MMsgHdr,
    vlen: c_uint,
    flags: c_int,
) -> c_int {
    // SAFETY: forwarded from this function's own contract.
    unsafe { send_mmsg_inner(sockfd, msgvec, vlen, flags) }
}

/// Receives up to `vlen` messages in one request.
///
/// # Safety
///
/// `msgvec` must point to `vlen` readable and writable [`MMsgHdr`]s whose buffers are valid as
/// they describe; `timeout` must be null or point to a readable [`TimeSpec`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__recvmmsg(
    sockfd: c_int,
    msgvec: *mut MMsgHdr,
    vlen: c_uint,
    flags: c_int,
    timeout: *const TimeSpec,
) -> c_int {
    // SAFETY: forwarded from this function's own contract.
    unsafe { recv_mmsg_inner(sockfd, msgvec, vlen, flags, timeout) }
}

/// The body shared by `sendmsg` and `sendmmsg`.
///
/// # Safety
///
/// As [`__nx_sys_net__sendmmsg`].
unsafe fn send_mmsg_inner(
    sockfd: c_int,
    msgvec: *mut MMsgHdr,
    vlen: c_uint,
    flags: c_int,
) -> c_int {
    // SAFETY: the caller guarantees `vlen` valid entries at `msgvec`.
    let Some(messages) = (unsafe { as_mut_slice_of(msgvec, vlen) }) else {
        return errno::fail(errno::EINVAL);
    };

    // SAFETY: each entry's buffers are valid as the entry describes, per the caller's contract.
    let mut buf = match unsafe { pack(messages, Direction::Send) } {
        Ok(buf) => buf,
        Err(code) => return errno::fail(code),
    };

    let flags = SendFlags::from_bits_truncate(flags);
    let count = match with_socket(sockfd, |svc, sock| {
        svc.send_mmsg(sock, &mut buf, message_count(messages), flags)
    }) {
        Ok(count) => count,
        Err(failure) => return failure,
    };

    // SAFETY: as above.
    match unsafe { unpack(messages, &buf, Direction::Send) } {
        Ok(()) => count,
        Err(code) => errno::fail(code),
    }
}

/// The body shared by `recvmsg` and `recvmmsg`.
///
/// # Safety
///
/// As [`__nx_sys_net__recvmmsg`].
unsafe fn recv_mmsg_inner(
    sockfd: c_int,
    msgvec: *mut MMsgHdr,
    vlen: c_uint,
    flags: c_int,
    timeout: *const TimeSpec,
) -> c_int {
    // SAFETY: the caller guarantees `vlen` valid entries at `msgvec`.
    let Some(messages) = (unsafe { as_mut_slice_of(msgvec, vlen) }) else {
        return errno::fail(errno::EINVAL);
    };

    // SAFETY: each entry's buffers are valid as the entry describes, per the caller's contract.
    let mut buf = match unsafe { pack(messages, Direction::Receive) } {
        Ok(buf) => buf,
        Err(code) => return errno::fail(code),
    };

    let timeout = if timeout.is_null() {
        // The interface's "wait indefinitely" sentinel, which is what a null timeout means.
        RecvTimeout { sec: -1, nsec: 0 }
    } else {
        // SAFETY: the caller guarantees a readable value at a non-null pointer.
        let spec = unsafe { *timeout };
        RecvTimeout {
            sec: spec.tv_sec,
            nsec: spec.tv_nsec,
        }
    };

    let flags = RecvFlags::from_bits_truncate(flags);
    let count = match with_socket(sockfd, |svc, sock| {
        svc.recv_mmsg(sock, &mut buf, message_count(messages), flags, timeout)
    }) {
        Ok(count) => count,
        Err(failure) => return failure,
    };

    // SAFETY: as above.
    match unsafe { unpack(messages, &buf, Direction::Receive) } {
        Ok(()) => count,
        Err(code) => errno::fail(code),
    }
}

/// Which way a message vector is travelling.
///
/// The packed layout is the same either way; what differs is whether the payloads are copied in
/// before the request or out after it. Naming the direction is what keeps [`pack`] and [`unpack`]
/// one function each rather than two near-identical pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// The caller's buffers hold data to be sent, so packing copies them in.
    Send,
    /// The caller's buffers are to be filled, so unpacking copies them out.
    Receive,
}

/// The largest total payload the service accepts across one message vector.
const MAX_TOTAL_PAYLOAD: usize = 0x80000;

/// Packs a message vector into the single contiguous request the service takes.
///
/// # Errors
///
/// Returns `EOPNOTSUPP` for ancillary data the service will not carry, `EMSGSIZE` when the
/// payloads exceed [`MAX_TOTAL_PAYLOAD`], and `ENOMEM` when the request buffer cannot be
/// allocated.
///
/// # Safety
///
/// Every entry's `msg_name`, `msg_iov` and `msg_control` must be valid for the lengths the entry
/// declares.
unsafe fn pack(messages: &[MMsgHdr], direction: Direction) -> Result<Vec<u8>, c_int> {
    let mut size = 1usize;
    let mut payload_total = 0usize;

    for entry in messages {
        // SAFETY: the caller vouches for the entry's buffers and counts.
        let iov = unsafe { iov_of(&entry.msg_hdr) };
        // SAFETY: as above.
        unsafe { reject_unsupported_control(&entry.msg_hdr)? };

        size += size_of::<SockLenT>() + entry.msg_hdr.msg_namelen as usize + size_of::<c_int>();
        for vec in iov {
            size += size_of::<u64>() + vec.iov_len;
            payload_total += vec.iov_len;
        }
        size += size_of::<SockLenT>()
            + entry.msg_hdr.msg_controllen as usize
            + size_of::<c_int>()
            + size_of::<c_int>();
    }

    if payload_total > MAX_TOTAL_PAYLOAD {
        return Err(errno::EMSGSIZE);
    }

    let mut buf = vec![0u8; size];
    let mut at = 0usize;

    // The leading byte the service expects ahead of the first message.
    buf[at] = 0x8;
    at += 1;

    for entry in messages {
        let header = &entry.msg_hdr;
        // SAFETY: the caller vouches for the entry's buffers and counts.
        let iov = unsafe { iov_of(header) };

        put_u32(&mut buf, &mut at, header.msg_namelen);
        if direction == Direction::Send && !header.msg_name.is_null() {
            // SAFETY: the caller vouches for `msg_namelen` readable bytes at `msg_name`.
            let name = unsafe {
                core::slice::from_raw_parts(
                    header.msg_name.cast::<u8>(),
                    header.msg_namelen as usize,
                )
            };
            buf[at..at + name.len()].copy_from_slice(name);
        }
        at += header.msg_namelen as usize;

        put_i32(&mut buf, &mut at, header.msg_iovlen);
        for vec in iov {
            put_u64(&mut buf, &mut at, vec.iov_len as u64);
            if direction == Direction::Send {
                // SAFETY: the caller vouches for `iov_len` readable bytes at `iov_base`.
                let payload =
                    unsafe { core::slice::from_raw_parts(vec.iov_base.cast::<u8>(), vec.iov_len) };
                buf[at..at + payload.len()].copy_from_slice(payload);
            }
            at += vec.iov_len;
        }

        put_u32(&mut buf, &mut at, header.msg_controllen);
        if direction == Direction::Send && !header.msg_control.is_null() {
            // SAFETY: the caller vouches for `msg_controllen` readable bytes at `msg_control`.
            let control = unsafe {
                core::slice::from_raw_parts(
                    header.msg_control.cast::<u8>(),
                    header.msg_controllen as usize,
                )
            };
            buf[at..at + control.len()].copy_from_slice(control);
        }
        at += header.msg_controllen as usize;

        put_i32(&mut buf, &mut at, header.msg_flags);
        // Bounded by `MAX_TOTAL_PAYLOAD`, which `pack` refused to exceed before allocating.
        put_i32(&mut buf, &mut at, entry.msg_len as c_int);
    }

    Ok(buf)
}

/// Unpacks the service's response back into the caller's message vector.
///
/// Every length read from `buf` is checked against two bounds before it is used: the end of the
/// response, and what the caller's own structure declared. The second is the one that matters —
/// a length the service reported larger than the caller's buffer would otherwise be a write past
/// the end of it.
///
/// # Errors
///
/// Returns `EFAULT` when the response is malformed or describes more than the caller's structures
/// can hold, and `EOPNOTSUPP` for ancillary data the service will not carry.
///
/// # Safety
///
/// As [`pack`].
unsafe fn unpack(messages: &mut [MMsgHdr], buf: &[u8], direction: Direction) -> Result<(), c_int> {
    // Skip the leading byte written by `pack`.
    let mut at = 1usize;

    for entry in messages.iter_mut() {
        let name_len = take_u32(buf, &mut at)?;
        if name_len > entry.msg_hdr.msg_namelen {
            return Err(errno::EFAULT);
        }
        entry.msg_hdr.msg_namelen = name_len;

        let name = take_bytes(buf, &mut at, name_len as usize)?;
        if direction == Direction::Receive && !entry.msg_hdr.msg_name.is_null() {
            // SAFETY: `name_len` is no larger than the length the caller declared for the buffer.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    name.as_ptr(),
                    entry.msg_hdr.msg_name.cast::<u8>(),
                    name.len(),
                )
            };
        }

        let iov_len = take_i32(buf, &mut at)?;
        if iov_len > entry.msg_hdr.msg_iovlen || iov_len < 0 {
            return Err(errno::EFAULT);
        }
        entry.msg_hdr.msg_iovlen = iov_len;

        // SAFETY: the caller vouches for the entry's `iovec` array, and `iov_len` was just checked
        // against the count the entry declared.
        let iov = unsafe {
            core::slice::from_raw_parts_mut(entry.msg_hdr.msg_iov, iov_len.unsigned_abs() as usize)
        };

        for vec in iov {
            // Narrowed before it is checked, and the check on the next line is what bounds it:
            // a value too large for `usize` cannot survive it either.
            let payload_len = take_u64(buf, &mut at)? as usize;
            if payload_len > vec.iov_len {
                return Err(errno::EFAULT);
            }
            let payload = take_bytes(buf, &mut at, payload_len)?;
            vec.iov_len = payload_len;
            if direction == Direction::Receive {
                // SAFETY: `payload_len` is no larger than the region the caller declared.
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        payload.as_ptr(),
                        vec.iov_base.cast::<u8>(),
                        payload.len(),
                    )
                };
            }
        }

        let control_len = take_u32(buf, &mut at)?;
        if control_len > entry.msg_hdr.msg_controllen {
            return Err(errno::EFAULT);
        }
        entry.msg_hdr.msg_controllen = control_len;

        let control = take_bytes(buf, &mut at, control_len as usize)?;
        if direction == Direction::Receive && !entry.msg_hdr.msg_control.is_null() {
            // SAFETY: `control_len` is no larger than the length the caller declared.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    control.as_ptr(),
                    entry.msg_hdr.msg_control.cast::<u8>(),
                    control.len(),
                )
            };
        }

        entry.msg_hdr.msg_flags = take_i32(buf, &mut at)?;
        entry.msg_len = take_i32(buf, &mut at)? as SsizeT;

        // SAFETY: the control buffer is whatever the caller supplied, now filled by the service.
        unsafe { reject_unsupported_control(&entry.msg_hdr)? };
    }

    Ok(())
}

/// Refuses the one ancillary element the service cannot carry.
///
/// A `SOL_SOCKET`-level rights transfer would hand a descriptor across a process boundary, which
/// this platform has no equivalent for.
///
/// # Safety
///
/// `header.msg_control` must be null or point to at least `msg_controllen` readable bytes.
unsafe fn reject_unsupported_control(header: &MsgHdr) -> Result<(), c_int> {
    /// `SOL_SOCKET`, as the ancillary level.
    const SOL_SOCKET: c_int = 0xFFFF;
    /// `SCM_RIGHTS`.
    const SCM_RIGHTS: c_int = 1;

    if (header.msg_controllen as usize) < size_of::<CMsgHdr>() || header.msg_control.is_null() {
        return Ok(());
    }

    // SAFETY: the caller guarantees at least `msg_controllen` readable bytes, which the check
    // above proved is at least one header's worth.
    let cmsg = unsafe { *header.msg_control.cast::<CMsgHdr>() };
    if cmsg.cmsg_level == SOL_SOCKET && cmsg.cmsg_type == SCM_RIGHTS {
        return Err(errno::EOPNOTSUPP);
    }

    Ok(())
}

/// Borrows an entry's scatter-gather array.
///
/// # Safety
///
/// `header.msg_iov` must point to `msg_iovlen` readable elements, or `msg_iovlen` must be zero.
unsafe fn iov_of(header: &MsgHdr) -> &[super::abi::IoVec] {
    if header.msg_iov.is_null() || header.msg_iovlen <= 0 {
        return &[];
    }

    // SAFETY: the caller guarantees the element count declared by the header.
    unsafe {
        core::slice::from_raw_parts(header.msg_iov, header.msg_iovlen.unsigned_abs() as usize)
    }
}

/// Writes a 32-bit value and advances the cursor.
fn put_u32(buf: &mut [u8], at: &mut usize, value: u32) {
    buf[*at..*at + 4].copy_from_slice(&value.to_ne_bytes());
    *at += 4;
}

/// Writes a signed 32-bit value and advances the cursor.
fn put_i32(buf: &mut [u8], at: &mut usize, value: i32) {
    buf[*at..*at + 4].copy_from_slice(&value.to_ne_bytes());
    *at += 4;
}

/// Writes a 64-bit value and advances the cursor.
fn put_u64(buf: &mut [u8], at: &mut usize, value: u64) {
    buf[*at..*at + 8].copy_from_slice(&value.to_ne_bytes());
    *at += 8;
}

/// Reads a 32-bit value, refusing to run off the end.
fn take_u32(buf: &[u8], at: &mut usize) -> Result<u32, c_int> {
    let bytes = take_bytes(buf, at, 4)?;
    Ok(u32::from_ne_bytes(bytes.try_into().unwrap_or([0; 4])))
}

/// Reads a signed 32-bit value, refusing to run off the end.
fn take_i32(buf: &[u8], at: &mut usize) -> Result<i32, c_int> {
    let bytes = take_bytes(buf, at, 4)?;
    Ok(i32::from_ne_bytes(bytes.try_into().unwrap_or([0; 4])))
}

/// Reads a 64-bit value, refusing to run off the end.
fn take_u64(buf: &[u8], at: &mut usize) -> Result<u64, c_int> {
    let bytes = take_bytes(buf, at, 8)?;
    Ok(u64::from_ne_bytes(bytes.try_into().unwrap_or([0; 8])))
}

/// Borrows `len` bytes at the cursor and advances it.
///
/// # Errors
///
/// Returns `EFAULT` when the response does not hold that many bytes, which is the answer to every
/// malformed response: the caller's structures are left however far the unpack had got, and the
/// call reports failure.
fn take_bytes<'b>(buf: &'b [u8], at: &mut usize, len: usize) -> Result<&'b [u8], c_int> {
    let end = at.checked_add(len).ok_or(errno::EFAULT)?;
    let bytes = buf.get(*at..end).ok_or(errno::EFAULT)?;
    *at = end;
    Ok(bytes)
}

/// Reports how many messages the vector holds, as the commands count them.
///
/// The length is the `vlen` the caller passed, which arrived as an unsigned C `int` and so already
/// fits a signed one; a vector longer than that cannot be constructed here.
fn message_count(messages: &[MMsgHdr]) -> i32 {
    i32::try_from(messages.len()).unwrap_or(i32::MAX)
}

/// Reports a C failure return as C's signed size.
///
/// `isize` has no `From<i32>`, because it is not required to be wider on every target. On this one
/// it is, and every value reaching here is a small negative these calls produced themselves, so the
/// widening sign-extends rather than reinterpreting anything.
fn to_failure(code: c_int) -> SsizeT {
    code as SsizeT
}

/// Reports a byte count as C's signed size.
///
/// The count came back from a command that was handed one of the caller's own buffers, so it never
/// exceeds that buffer's length; a length a caller can address always fits the signed width, so the
/// narrowing cannot wrap into a value C would read as failure.
fn to_ssize(count: usize) -> SsizeT {
    count as SsizeT
}

/// Borrows a caller's buffer for reading.
///
/// # Safety
///
/// `ptr` must be null or point to at least `len` readable bytes.
unsafe fn as_slice<'a>(ptr: *const c_void, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return (len == 0).then_some(&[]);
    }
    // SAFETY: the caller guarantees `len` readable bytes.
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

/// Borrows a caller's buffer for writing.
///
/// # Safety
///
/// `ptr` must be null or point to at least `len` writable bytes.
unsafe fn as_mut_slice<'a>(ptr: *mut c_void, len: usize) -> Option<&'a mut [u8]> {
    if ptr.is_null() {
        return (len == 0).then_some(&mut []);
    }
    // SAFETY: the caller guarantees `len` writable bytes.
    Some(unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), len) })
}

/// Borrows a caller's message array.
///
/// # Safety
///
/// `ptr` must point to `count` readable and writable [`MMsgHdr`]s.
unsafe fn as_mut_slice_of<'a>(ptr: *mut MMsgHdr, count: c_uint) -> Option<&'a mut [MMsgHdr]> {
    if ptr.is_null() || count == 0 {
        return None;
    }
    // SAFETY: the caller guarantees `count` valid entries.
    Some(unsafe { core::slice::from_raw_parts_mut(ptr, count as usize) })
}
