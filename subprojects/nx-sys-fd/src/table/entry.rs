//! What an open descriptor holds.
//!
//! A descriptor always names a device. Whether it also owns an open [`File`] is what separates the
//! two ways a descriptor reaches that device:
//!
//! - **Bound without a path.** No file, so every operation resolves the registry afresh. This is
//!   how the standard descriptors work, and it is what lets a console take slot 1 over from the
//!   null device without reopening anything.
//! - **Opened by path.** The descriptor owns the file the device produced, and every operation goes
//!   to that object.
//!
//! The open file is held behind a lock of its own rather than under the table lock. A file
//! operation can block for as long as the underlying storage takes, and holding the table lock
//! across it would stall every unrelated descriptor and risk a deadlock against a device whose own
//! path reaches back into the table. So a caller clones the handle out under the table lock,
//! releases it, and only then locks the file.
//!
//! ## Two counts, two questions
//!
//! Duplication lets several descriptors name one open file, and that raises a question the
//! [`Arc`](alloc::sync::Arc) around it cannot answer. The `Arc` counts everything holding the file
//! alive, which includes the handle an in-flight write cloned out of the table; the device's close
//! must run when the last *descriptor* goes, not when the last holder does. So the descriptors are
//! counted separately, in [`OpenFile::links`]: the `Arc` decides when the object is freed, the link
//! count decides when the device is told.

use alloc::boxed::Box;
use core::{
    cell::UnsafeCell,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};

use nx_sys_sync::Mutex;

use crate::device::File;

/// An open file, and the lock ordering access to it.
///
/// Shared through an [`Arc`](alloc::sync::Arc) so that a caller can hold the file alive after
/// releasing the table lock. The last handle to go releases it, which is why closing a descriptor
/// never drops a file under the table lock even when another thread is mid-operation.
pub struct OpenFile {
    mutex: Mutex,
    file: UnsafeCell<Box<dyn File>>,
    links: AtomicU32,
}

// SAFETY: `file` is only reached through `lock`, which holds `mutex` for the life of the guard it
// returns, and the box is never moved out.
unsafe impl Sync for OpenFile {}

// SAFETY: `File` is itself `Send`, and the `UnsafeCell` adds no thread affinity of its own.
unsafe impl Send for OpenFile {}

impl OpenFile {
    /// Wraps `file` so it can be shared between the table and the callers operating on it.
    ///
    /// Starts with one link, for the descriptor this file is about to be attached to.
    pub fn new(file: Box<dyn File>) -> Self {
        Self {
            mutex: Mutex::new(),
            file: UnsafeCell::new(file),
            links: AtomicU32::new(1),
        }
    }

    /// Records that one more descriptor names this file.
    pub fn add_link(&self) {
        // Relaxed: the caller holds the table lock, so nothing observes the count out of order, and
        // the descriptor being added cannot be closed before it exists.
        self.links.fetch_add(1, Ordering::Relaxed);
    }

    /// Drops one descriptor's claim on this file, reporting whether it was the last.
    ///
    /// The last one is what decides the close: a file still named by another descriptor must not be
    /// told to close, however the one being released was reached.
    pub fn remove_link(&self) -> bool {
        // AcqRel: the caller that sees the count reach zero closes the file, so it must observe
        // everything every other link did before letting go of it.
        self.links.fetch_sub(1, Ordering::AcqRel) == 1
    }

    /// Locks the file for the lifetime of the returned guard.
    ///
    /// Blocks while another caller operates on the same descriptor, which is what serializes two
    /// threads writing the same open file.
    pub fn lock(&self) -> FileGuard<'_> {
        self.mutex.lock();
        FileGuard(self)
    }
}

/// Exclusive access to an open file, unlocking on drop.
pub struct FileGuard<'a>(&'a OpenFile);

impl FileGuard<'_> {
    /// Returns the file this guard has exclusive access to.
    pub fn file(&mut self) -> &mut dyn File {
        // SAFETY: holding this guard means the file's lock is held, so no other reference exists.
        unsafe { &mut **self.0.file.get() }
    }
}

impl Drop for FileGuard<'_> {
    fn drop(&mut self) {
        self.0.mutex.unlock();
    }
}
