//! The image a homebrew `NRO` carries inside itself.
//!
//! An `NRO` that ships data has it appended past the end of its own code, behind an asset header
//! that says where each piece starts. So mounting "my own romfs" here means opening the very file
//! this process was launched from, reading two headers out of it, and handing the rest to
//! [`nx_romfs`].
//!
//! ## Why this is the runtime's job
//!
//! libnx answers the same question with `romfsMountSelf`, which asks whether the process is an
//! `NSO` and mounts its data partition if so, or reads the appended image if not. Nothing below the
//! runtime may ask that: the output kind is settled by which entry crate a binary links, so the
//! branch is not a branch at all once the code sits in the right crate. This module is the `NRO`
//! answer, and it takes the appended-image route unconditionally.
//!
//! ## Where the file name comes from
//!
//! From `argv[0]`, which the loader fills in with the path it launched. It reaches here through
//! [`nx_sys_args`], where the startup sequence installed it, rather than through the C globals
//! libnx reads: those hold whatever the last library to write them put there.
//!
//! libnx also honours a weak `__romfs_path` global a program may define to name its own file when
//! `argv` is empty. Nothing supplies one in this workspace, and a program whose loader gave it no
//! arguments has no image to find, so the fallback is left out rather than reintroduced as a knob.

use alloc::boxed::Box;

use nx_object::raw::nro::{
    ASSET_MAGIC,
    NRO_MAGIC,
    NroAssetHeader,
    NroHeader,
    NroStart,
};
use nx_std_path::{
    OsStr,
    Path,
};
use nx_sys_fd::{
    device::{
        DeviceError,
        File,
        OpenFlags,
        SeekFrom,
    },
    path,
    registry,
};
use zerocopy::FromBytes as _;

/// The newest revision of the asset header this crate knows how to read.
const ASSET_VERSION: u32 = 0;

/// Mounts this program's own image under `name`.
///
/// # Errors
///
/// Returns [`MountSelfError::NoProgramPath`] when the loader passed no path to launch from,
/// [`MountSelfError::NoDevice`] when no mounted device serves it, [`MountSelfError::Open`] when the
/// file could not be opened, [`MountSelfError::NotAnNro`] and [`MountSelfError::NoAssets`] when the
/// file is not an `NRO` carrying an image, and [`MountSelfError::Mount`] when the image was found
/// but could not be mounted.
pub fn mount_self(name: &str) -> Result<(), MountSelfError> {
    let program_path = nx_sys_args::args()
        .next()
        .ok_or(MountSelfError::NoProgramPath)?;
    let program_path = Path::new(OsStr::from_bytes(program_path));

    let mut file = open(program_path)?;
    let offset = romfs_offset(file.as_mut())?;

    // The file that was read is the file that is mounted, so the image is reached through the one
    // descriptor rather than opened a second time.
    nx_romfs::mount::from_device_file(name, file, offset).map_err(MountSelfError::Mount)
}

/// Errors returned by [`mount_self`].
#[derive(Debug, thiserror::Error)]
pub enum MountSelfError {
    /// The loader passed no path to launch from
    ///
    /// Occurs when the command line is empty, which leaves nothing naming the file this image would
    /// be read out of. Nothing was opened.
    #[error("the command line names no program to read the image from")]
    NoProgramPath,

    /// No mounted device serves that path
    ///
    /// Occurs when the image is mounted before the SD card is, or the path's prefix names a device
    /// that is gone. Nothing was opened.
    #[error("no mounted device serves the program's own path")]
    NoDevice,

    /// The program's own file could not be opened
    ///
    /// Occurs when the file has moved or the device refused to open it. Nothing was mounted.
    #[error("failed to open the program's own file")]
    Open(#[source] DeviceError),

    /// The file is not an `NRO`
    ///
    /// Occurs when the header's magic is something else, which means the command line names a file
    /// other than the one running. Nothing was mounted.
    #[error("the program's own file is not an NRO")]
    NotAnNro,

    /// The `NRO` carries no image
    ///
    /// Occurs when nothing was appended, the appended block is a revision this crate cannot read,
    /// or it holds an icon and control data but no romfs. This is the ordinary answer for a program
    /// built without one.
    #[error("the NRO carries no romfs image")]
    NoAssets,

    /// The image was found but could not be mounted
    ///
    /// Occurs when the name is taken, the bytes at that offset are not an image, or the descriptor
    /// table is full. The file was closed.
    #[error("failed to mount the image")]
    Mount(#[source] nx_romfs::mount::MountError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for MountSelfError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_rt_core::error::{
            LibnxError,
            libnx_error,
        };

        match self {
            // Nothing to read the image out of, which is the same answer libnx gives when the
            // command line names no file and no override supplies one.
            Self::NoProgramPath | Self::NoDevice => libnx_error(LibnxError::NotFound),
            Self::Open(_) => libnx_error(LibnxError::NotFound),
            // libnx reports both of these as an I/O failure, and a caller that branches on the code
            // to decide whether the program simply has no data must not have to learn a new one.
            Self::NotAnNro | Self::NoAssets => libnx_error(LibnxError::IoError),
            Self::Mount(err) => match err {
                nx_romfs::mount::MountError::AlreadyMounted
                | nx_romfs::mount::MountError::Registry(_) => libnx_error(LibnxError::OutOfMemory),
                nx_romfs::mount::MountError::Image(_) => libnx_error(LibnxError::IoError),
            },
        }
    }
}

/// Opens `path` on whichever mounted device serves it, for reading.
fn open(path: &Path) -> Result<Box<dyn File>, MountSelfError> {
    let id = path::device_for_path(path).ok_or(MountSelfError::NoDevice)?;
    let device = registry::get(id).ok_or(MountSelfError::NoDevice)?;

    device
        .open(path::strip_device_prefix(path), READ_ONLY)
        .map_err(MountSelfError::Open)
}

/// How the program's own file is opened: for reading, and nothing else.
const READ_ONLY: OpenFlags = OpenFlags {
    read: true,
    write: false,
    append: false,
    create: false,
    exclusive: false,
    truncate: false,
};

/// Returns how far into `file` the appended romfs image starts.
///
/// Reads the `NRO` header to find where the code ends, then the asset header sitting there to find
/// where the image sits inside the appended block.
fn romfs_offset(file: &mut dyn File) -> Result<u64, MountSelfError> {
    let mut header_bytes = [0u8; size_of::<NroHeader>()];
    read_exact_at(file, size_of::<NroStart>() as u64, &mut header_bytes)
        .map_err(|_| MountSelfError::NotAnNro)?;
    // The buffer is exactly the header's size and the header is all byte-order fields, so the only
    // way this fails is a length mismatch that cannot happen here.
    let Ok(header) = NroHeader::read_from_bytes(&header_bytes) else {
        return Err(MountSelfError::NotAnNro);
    };
    if header.magic.get() != NRO_MAGIC {
        return Err(MountSelfError::NotAnNro);
    }

    // Everything past this point is the ordinary shape of a program built without an image, so a
    // file that simply ends here is not a failure to report differently.
    let assets_at = u64::from(header.size.get());
    let mut assets_bytes = [0u8; size_of::<NroAssetHeader>()];
    read_exact_at(file, assets_at, &mut assets_bytes).map_err(|_| MountSelfError::NoAssets)?;
    let Ok(assets) = NroAssetHeader::read_from_bytes(&assets_bytes) else {
        return Err(MountSelfError::NoAssets);
    };
    if assets.magic.get() != ASSET_MAGIC || assets.version.get() > ASSET_VERSION {
        return Err(MountSelfError::NoAssets);
    }
    // A program built without an image leaves the section empty rather than omitting the header, so
    // this is the ordinary "no romfs here" answer and not a malformed file.
    if assets.romfs.offset.get() == 0 || assets.romfs.size.get() == 0 {
        return Err(MountSelfError::NoAssets);
    }

    assets_at
        .checked_add(assets.romfs.offset.get())
        .ok_or(MountSelfError::NoAssets)
}

/// Fills `buf` from `offset` bytes into `file`.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when the file ends before `buf` is full, which is what a file with
/// nothing appended looks like from here, and whatever the device rejected the read with.
fn read_exact_at(file: &mut dyn File, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
    file.seek(SeekFrom::Start(offset))?;

    let mut filled = 0;
    while filled < buf.len() {
        let read = file.read(&mut buf[filled..])?;
        if read == 0 {
            return Err(DeviceError::Io);
        }
        filled += read;
    }

    Ok(())
}
