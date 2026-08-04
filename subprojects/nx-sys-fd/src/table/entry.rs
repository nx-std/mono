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

use alloc::boxed::Box;
use core::cell::UnsafeCell;

use nx_sys_sync::Mutex;

use crate::device::File;

/// An open file, and the lock ordering access to it.
///
/// Shared through an [`Arc`] so that a caller can hold the file alive after releasing the table
/// lock. The last handle to go releases it, which is why closing a descriptor never drops a file
/// under the table lock even when another thread is mid-operation.
pub struct OpenFile {
    mutex: Mutex,
    file: UnsafeCell<Box<dyn File>>,
}

// SAFETY: `file` is only reached through `lock`, which holds `mutex` for the life of the guard it
// returns, and the box is never moved out.
unsafe impl Sync for OpenFile {}

// SAFETY: `File` is itself `Send`, and the `UnsafeCell` adds no thread affinity of its own.
unsafe impl Send for OpenFile {}

impl OpenFile {
    /// Wraps `file` so it can be shared between the table and the callers operating on it.
    pub fn new(file: Box<dyn File>) -> Self {
        Self {
            mutex: Mutex::new(),
            file: UnsafeCell::new(file),
        }
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
