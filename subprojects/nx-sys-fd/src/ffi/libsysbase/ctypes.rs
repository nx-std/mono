//! The C declarations this boundary speaks in.
//!
//! Every type here mirrors one the C standard library already declares, so the layouts are fixed by
//! that declaration rather than chosen. Each is pinned with a size assertion, and the structures the
//! entry points write through are pinned field by field: a mismatch would otherwise scribble over a
//! caller's stack in a way no test on this side would notice.
//!
//! The offsets were read out of the toolchain rather than inferred from the headers, because several
//! of these fields are narrower than their names suggest. `st_dev`, `st_ino`, `st_nlink`, `st_uid`,
//! `st_gid` and `st_rdev` are all 16 bits here, and `struct statvfs` mixes 64-bit block counts with
//! 32-bit file counts.
//!
//! # References
//!
//! - newlib/libc/include/sys/stat.h
//! - newlib/libc/include/sys/statvfs.h
//! - libgloss/libsysbase/iosupport.h

use core::ffi::{
    c_int,
    c_long,
    c_void,
};

use crate::device::{
    FileType,
    MAX_NAME_LEN,
    Metadata,
    OpenFlags,
    SpaceInfo,
};

/// File offset, matching the C library's `off_t`.
pub type OffT = c_long;

/// File mode, matching the C library's `mode_t`.
pub type ModeT = u32;

/// Signed byte count, matching the C library's `ssize_t`.
pub type SsizeT = c_long;

/// File-type bits marking a directory.
const S_IFDIR: ModeT = 0o040000;

/// File-type bits marking a regular file.
const S_IFREG: ModeT = 0o100000;

/// Permission bits reported for a regular file: readable and writable by everyone.
///
/// Horizon has no permission bits of its own, so these are invented. They are what the C
/// implementation reported, and a program that checks them before opening a file would otherwise
/// refuse to.
const FILE_PERMISSIONS: ModeT = 0o666;

/// Permission bits reported for a directory, invented for the same reason as [`FILE_PERMISSIONS`].
const DIR_PERMISSIONS: ModeT = 0o777;

/// A moment, as the C library declares `struct timespec`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TimeSpec {
    /// Seconds since the Unix epoch.
    pub tv_sec: i64,
    /// Nanoseconds within the second.
    pub tv_nsec: i64,
}

/// Entry metadata, as the C library declares `struct stat`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u16,
    pub st_ino: u16,
    pub st_mode: ModeT,
    pub st_nlink: u16,
    pub st_uid: u16,
    pub st_gid: u16,
    pub st_rdev: u16,
    pub st_size: i64,
    pub st_atim: TimeSpec,
    pub st_mtim: TimeSpec,
    pub st_ctim: TimeSpec,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_spare4: [c_long; 2],
}

// The C entry points hand out a pointer to one of these and read the fields back by offset. Pin
// every offset the boundary writes, so a change here is a build failure rather than a caller's
// stack quietly overwritten.
static_assertions::assert_eq_size!(Stat, [u8; 104]);
const _: () = {
    assert!(core::mem::offset_of!(Stat, st_dev) == 0);
    assert!(core::mem::offset_of!(Stat, st_ino) == 2);
    assert!(core::mem::offset_of!(Stat, st_mode) == 4);
    assert!(core::mem::offset_of!(Stat, st_nlink) == 8);
    assert!(core::mem::offset_of!(Stat, st_uid) == 10);
    assert!(core::mem::offset_of!(Stat, st_gid) == 12);
    assert!(core::mem::offset_of!(Stat, st_rdev) == 14);
    assert!(core::mem::offset_of!(Stat, st_size) == 16);
    assert!(core::mem::offset_of!(Stat, st_atim) == 24);
    assert!(core::mem::offset_of!(Stat, st_mtim) == 40);
    assert!(core::mem::offset_of!(Stat, st_ctim) == 56);
    assert!(core::mem::offset_of!(Stat, st_blksize) == 72);
    assert!(core::mem::offset_of!(Stat, st_blocks) == 80);
};

impl From<Metadata> for Stat {
    fn from(metadata: Metadata) -> Self {
        let (kind, permissions) = match metadata.file_type {
            FileType::File => (S_IFREG, FILE_PERMISSIONS),
            FileType::Dir => (S_IFDIR, DIR_PERMISSIONS),
        };

        // A link count of zero reads as an unlinked entry, and callers treat that as deleted.
        // Horizon has no hard links, so every entry that exists has exactly one.
        let mut stat = Self {
            st_mode: kind | permissions,
            st_nlink: 1,
            // A lossy cast: sizes beyond 2^63 do not occur on any Horizon filesystem, and the C
            // field is signed regardless, so there is nowhere wider to report them.
            st_size: metadata.size as i64,
            ..Self::default()
        };

        if let Some(timestamps) = metadata.timestamps {
            // Another lossy cast, on the same reasoning: these are seconds since 1970.
            stat.st_ctim.tv_sec = timestamps.created as i64;
            stat.st_mtim.tv_sec = timestamps.modified as i64;
            stat.st_atim.tv_sec = timestamps.accessed as i64;
        }

        stat
    }
}

/// Filesystem capacity, as the C library declares `struct statvfs`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct StatVfs {
    pub f_bsize: u64,
    pub f_frsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u32,
    pub f_ffree: u32,
    pub f_favail: u32,
    pub f_fsid: u64,
    pub f_flag: u64,
    pub f_namemax: u64,
}

// Pinned for the same reason as `Stat`, and with the same consequence for getting it wrong. Note
// that the file counts are half the width of the block counts.
static_assertions::assert_eq_size!(StatVfs, [u8; 80]);
const _: () = {
    assert!(core::mem::offset_of!(StatVfs, f_bsize) == 0);
    assert!(core::mem::offset_of!(StatVfs, f_frsize) == 8);
    assert!(core::mem::offset_of!(StatVfs, f_blocks) == 16);
    assert!(core::mem::offset_of!(StatVfs, f_bfree) == 24);
    assert!(core::mem::offset_of!(StatVfs, f_bavail) == 32);
    assert!(core::mem::offset_of!(StatVfs, f_files) == 40);
    assert!(core::mem::offset_of!(StatVfs, f_ffree) == 44);
    assert!(core::mem::offset_of!(StatVfs, f_favail) == 48);
    assert!(core::mem::offset_of!(StatVfs, f_fsid) == 56);
    assert!(core::mem::offset_of!(StatVfs, f_flag) == 64);
    assert!(core::mem::offset_of!(StatVfs, f_namemax) == 72);
};

impl From<SpaceInfo> for StatVfs {
    fn from(info: SpaceInfo) -> Self {
        Self {
            f_bsize: info.block_size,
            f_frsize: info.block_size,
            f_blocks: info.total_blocks,
            f_bfree: info.free_blocks,
            // Horizon reserves nothing for the superuser, so what is free is what is available.
            f_bavail: info.free_blocks,
            // A widening cast from a compile-time constant of 255.
            f_namemax: MAX_NAME_LEN as u64,
            ..Self::default()
        }
    }
}

/// Opaque `struct timeval`.
///
/// Only ever passed through: `utimes` is not implemented, so nothing here reads the fields.
#[repr(C)]
pub struct TimeVal {
    _opaque: [u8; 0],
}

/// Directory iteration state carried between `dir*` calls.
///
/// Mirrors `DIR_ITER` from `sys/iosupport.h`.
#[repr(C)]
pub struct DirIter {
    /// Registry slot of the device backing this iterator.
    pub device: c_int,
    /// The device's private directory state.
    pub dir_struct: *mut c_void,
}

// The C caller allocates this and reads `device` back itself, so the layout is not ours to choose.
static_assertions::assert_eq_size!(DirIter, [u64; 2]);
const _: () = {
    assert!(core::mem::offset_of!(DirIter, device) == 0);
    assert!(core::mem::offset_of!(DirIter, dir_struct) == 8);
};

/// Mask selecting the access mode from an `open(2)` flag word.
const O_ACCMODE: c_int = 0o3;
/// Access mode requesting reads only.
const O_RDONLY: c_int = 0o0;
/// Access mode requesting writes only.
const O_WRONLY: c_int = 0o1;
/// Access mode requesting both.
const O_RDWR: c_int = 0o2;

/// Create the entry when it does not exist.
const O_CREAT: c_int = 0o100;
/// Fail when the entry already exists.
const O_EXCL: c_int = 0o200;
/// Discard the existing contents on open.
const O_TRUNC: c_int = 0o1000;
/// Every write goes to the end of the file.
const O_APPEND: c_int = 0o2000;

/// Decodes an `open(2)` flag word into what the caller asked for.
///
/// An access mode the C library does not define resolves to neither read nor write, which every
/// operation then refuses. That is the honest reading: the caller asked for something that is not
/// an access mode.
pub fn decode_open_flags(flags: c_int) -> OpenFlags {
    let access = flags & O_ACCMODE;

    OpenFlags {
        read: access == O_RDONLY || access == O_RDWR,
        write: access == O_WRONLY || access == O_RDWR,
        append: flags & O_APPEND != 0,
        create: flags & O_CREAT != 0,
        exclusive: flags & O_EXCL != 0,
        truncate: flags & O_TRUNC != 0,
    }
}
