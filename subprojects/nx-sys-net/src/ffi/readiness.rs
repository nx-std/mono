//! Waiting for sockets to become ready.
//!
//! Both calls take the process's descriptor numbers and the service understands only its own, so
//! each entry is translated on the way in and the results are matched back on the way out. That
//! translation is the whole of the work; the waiting itself is one command.
//!
//! ## `select` is written on top of `poll`
//!
//! The service has a `select` command, and it is not the one to use. It takes the C `fd_set` byte
//! layout, which would mean rewriting three bitmaps in terms of service descriptors, sending them,
//! and mapping three more back — with no way to tell which process descriptor a set bit came from
//! once the numbers have changed. `poll` carries one descriptor per entry, so the correspondence
//! survives the round trip.
//!
//! The C driver reaches the same conclusion and says so in a comment. This follows it.

use alloc::vec::Vec;
use core::{
    ffi::c_int,
    time::Duration,
};

use nx_service_bsd::PollEvents;

use super::{
    abi::{
        FD_SETSIZE,
        FdSet,
        NfdsT,
        PollFd,
        TimeVal,
    },
    descriptor,
    errno,
};
use crate::session;

/// Waits for readiness across a descriptor array.
///
/// # Safety
///
/// `fds` must point to `nfds` readable and writable [`PollFd`]s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__poll(
    fds: *mut PollFd,
    nfds: NfdsT,
    timeout: c_int,
) -> c_int {
    if fds.is_null() && nfds != 0 {
        return errno::fail(errno::EFAULT);
    }
    if nfds == 0 {
        // Nothing to wait on. C says this is a plain sleep, and a zero count with no descriptors
        // has nothing that could become ready, so it reports that nothing did.
        return 0;
    }
    // `nfds_t` is unsigned and the answer is an `int`, so a set past `int` has no count this call
    // could report. Refused here rather than truncated, which would answer a different question.
    if c_int::try_from(nfds).is_err() {
        return errno::fail(errno::EINVAL);
    }

    // A negative timeout waits indefinitely, which the layer below spells as no timeout at all.
    let wait = match u64::try_from(timeout) {
        Ok(millis) => Some(Duration::from_millis(millis)),
        Err(_) => None,
    };

    // Exact: `nfds_t` is 32-bit unsigned and `usize` is 64-bit on this target.
    // SAFETY: the caller guarantees `nfds` valid entries at `fds`.
    let entries = unsafe { core::slice::from_raw_parts_mut(fds, nfds as usize) };
    poll_entries(entries, wait)
}

/// Waits for readiness across three descriptor sets.
///
/// # Safety
///
/// Each of `readfds`, `writefds` and `exceptfds` must be null or point to a readable and writable
/// [`FdSet`]; `timeout` must be null or point to a readable [`TimeVal`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__select(
    nfds: c_int,
    readfds: *mut FdSet,
    writefds: *mut FdSet,
    exceptfds: *mut FdSet,
    timeout: *mut TimeVal,
) -> c_int {
    let Ok(nfds) = usize::try_from(nfds) else {
        return errno::fail(errno::EINVAL);
    };
    if nfds > FD_SETSIZE {
        return errno::fail(errno::EINVAL);
    }

    // Build one poll entry per descriptor named in any of the three sets, remembering which set
    // asked for it so the answer can be put back where the caller will look.
    let mut entries: Vec<PollFd> = Vec::new();
    for number in 0..nfds {
        let mut events = PollEvents::empty();
        // SAFETY: the caller guarantees each non-null pointer refers to a readable set.
        if unsafe { set_contains(readfds, number) } {
            events |= PollEvents::IN;
        }
        // SAFETY: as above.
        if unsafe { set_contains(writefds, number) } {
            events |= PollEvents::OUT;
        }
        // SAFETY: as above.
        if unsafe { set_contains(exceptfds, number) } {
            events |= PollEvents::PRI;
        }

        if !events.is_empty() {
            entries.push(PollFd {
                // Bounded by `nfds`, which was refused above unless it is at most `FD_SETSIZE`.
                fd: number as c_int,
                events: events.bits(),
                revents: 0,
            });
        }
    }

    let wait = if timeout.is_null() {
        // Wait indefinitely, which is what a null timeout means.
        None
    } else {
        // SAFETY: the caller guarantees a readable value at a non-null pointer.
        let spec = unsafe { *timeout };
        match to_duration(spec) {
            Some(duration) => Some(duration),
            None => return errno::fail(errno::EINVAL),
        }
    };

    let ready = poll_entries(&mut entries, wait);
    if ready < 0 {
        return ready;
    }

    // The sets are rebuilt from scratch: C specifies that on return they name exactly the
    // descriptors that are ready, not the ones that were asked about.
    // SAFETY: the caller guarantees each non-null pointer refers to a writable set.
    unsafe { clear_set(readfds) };
    // SAFETY: as above.
    unsafe { clear_set(writefds) };
    // SAFETY: as above.
    unsafe { clear_set(exceptfds) };

    let mut count = 0;
    for entry in &entries {
        // Every entry here was built by the loop above, so the descriptor is one it wrote:
        // non-negative and below `FD_SETSIZE`.
        let number = entry.fd as usize;
        let mut named = false;

        let asked = PollEvents::from_bits_retain(entry.events);
        let reported = PollEvents::from_bits_retain(entry.revents);

        // An error, a hangup or an invalid descriptor is reported to whichever sets asked about
        // it, because C has no set of its own for them and a caller must not wait forever on a
        // descriptor that will never be ready.
        let failed = reported.intersects(PollEvents::ERR | PollEvents::HUP | PollEvents::NVAL);

        if asked.contains(PollEvents::IN) && (reported.contains(PollEvents::IN) || failed) {
            // SAFETY: the caller guarantees a writable set behind a non-null pointer.
            unsafe { insert_into_set(readfds, number) };
            named = true;
        }
        if asked.contains(PollEvents::OUT) && (reported.contains(PollEvents::OUT) || failed) {
            // SAFETY: as above.
            unsafe { insert_into_set(writefds, number) };
            named = true;
        }
        if asked.contains(PollEvents::PRI) && reported.contains(PollEvents::PRI) {
            // SAFETY: as above.
            unsafe { insert_into_set(exceptfds, number) };
            named = true;
        }

        if named {
            count += 1;
        }
    }

    count
}

/// Runs one poll command over `entries`, translating descriptors in both directions.
///
/// An entry naming something that is not a socket is not an error for the call as a whole: C says
/// it comes back with `POLLNVAL` set, and the remaining entries are waited on as asked. That is
/// why the translation collects a side list rather than returning early.
///
/// Both callers bound `entries` to what a `c_int` can count, which is what makes the returned count
/// representable: `poll` refuses a larger `nfds`, and `select` cannot name more than `FD_SETSIZE`
/// descriptors.
fn poll_entries(entries: &mut [PollFd], timeout: Option<Duration>) -> c_int {
    // The entries the service will actually be asked about, and where each came from.
    let mut wire: Vec<nx_service_bsd::PollFd> = Vec::with_capacity(entries.len());
    let mut origin: Vec<usize> = Vec::with_capacity(entries.len());

    for (index, entry) in entries.iter_mut().enumerate() {
        entry.revents = 0;

        // A negative descriptor asks for nothing, which C specifies is reported as nothing.
        if entry.fd < 0 {
            continue;
        }

        match descriptor::resolve(entry.fd) {
            Ok(sock) => {
                wire.push(nx_service_bsd::PollFd::new(
                    sock,
                    PollEvents::from_bits_retain(entry.events),
                ));
                origin.push(index);
            }
            Err(_) => entry.revents = PollEvents::NVAL.bits(),
        }
    }

    if wire.is_empty() {
        // Every entry was already answered, so there is nothing to wait for. Report how many.
        // Bounded by `entries.len()`, which both callers keep within a `c_int`.
        return entries.iter().filter(|e| e.revents != 0).count() as c_int;
    }

    let count = match session::with_service(|svc| svc.poll(&mut wire, timeout)) {
        Err(_) => return errno::fail(errno::EBADF),
        Ok(Err(err)) => return errno::report(err),
        Ok(Ok(count)) => count,
    };

    // Copy each answer back to the entry it came from.
    for (answered, &index) in wire.iter().zip(origin.iter()) {
        entries[index].revents = answered.revents().bits();
    }

    // The service counted only the entries it was given; the ones refused during translation are
    // ready too, in the sense C means, so they are added here.
    let refused = entries
        .iter()
        .filter(|e| e.revents == PollEvents::NVAL.bits())
        .count();

    // The two terms partition `entries`, so their sum is bounded by its length, which both callers
    // keep within a `c_int`.
    (count + refused) as c_int
}

/// Converts a `select` timeout into the wait the layer below takes.
///
/// Returns `None` for a negative component, which C rejects. A wait too long to express is clamped
/// rather than refused: the caller asked to wait a long time and gets the longest there is.
fn to_duration(timeout: TimeVal) -> Option<Duration> {
    let seconds = u64::try_from(timeout.tv_sec).ok()?;
    let micros = u64::try_from(timeout.tv_usec).ok()?;

    let seconds = Duration::from_secs(seconds);
    let micros = Duration::from_micros(micros);

    Some(seconds.checked_add(micros).unwrap_or(Duration::MAX))
}

/// Whether a set names `number`, treating a null set as naming nothing.
///
/// # Safety
///
/// `set` must be null or point to a readable [`FdSet`].
unsafe fn set_contains(set: *const FdSet, number: usize) -> bool {
    if set.is_null() {
        return false;
    }
    // SAFETY: the caller guarantees a readable set behind a non-null pointer.
    unsafe { (*set).contains(number) }
}

/// Empties a set, ignoring a null one.
///
/// # Safety
///
/// `set` must be null or point to a writable [`FdSet`].
unsafe fn clear_set(set: *mut FdSet) {
    if set.is_null() {
        return;
    }
    // SAFETY: the caller guarantees a writable set behind a non-null pointer.
    unsafe { (*set).clear() };
}

/// Adds `number` to a set, ignoring a null one.
///
/// # Safety
///
/// `set` must be null or point to a writable [`FdSet`].
unsafe fn insert_into_set(set: *mut FdSet, number: usize) {
    if set.is_null() {
        return;
    }
    // SAFETY: the caller guarantees a writable set behind a non-null pointer.
    unsafe { (*set).insert(number) };
}
