//! One open file on a mounted image.
//!
//! Where the file's contents start and how long they are were both settled when it was opened, out
//! of a record that cannot change afterwards. So a read here is arithmetic against those two
//! numbers followed by one read of the image, and nothing has to be looked up again.
//!
//! The image is held rather than borrowed, which is what lets the file outlive its mount: a
//! descriptor open when the device is unmounted keeps reading until it is closed.

use alloc::sync::Arc;

use nx_sys_fd::device::{
    DeviceError,
    File,
    Metadata,
    SeekFrom,
};

use crate::image::Image;

/// An open file, addressed by where its contents sit in the image.
pub(crate) struct RomfsFile {
    /// The image the contents are read from.
    image: Arc<Image>,
    /// Where the contents start, as an absolute offset into the image's container.
    offset: u64,
    /// How many bytes of contents there are.
    size: u64,
    /// Where the next read acts, in bytes from the start of the contents.
    pos: u64,
}

impl RomfsFile {
    /// Opens the `size` bytes of contents that start at `offset` in `image`.
    pub(crate) fn new(image: Arc<Image>, offset: u64, size: u64) -> Self {
        Self {
            image,
            offset,
            size,
            pos: 0,
        }
    }
}

impl File for RomfsFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        if self.pos >= self.size {
            return Ok(0);
        }

        // The read is clamped to what is left of the file, so the image is never asked for bytes
        // past the end of the contents and a short read from it is a genuine failure.
        let remaining = self.size - self.pos;
        let len = remaining.min(buf.len() as u64) as usize;

        self.image
            .read_contents(self.offset + self.pos, &mut buf[..len])?;

        self.pos += len as u64;
        Ok(len)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, DeviceError> {
        // Positions are measured from the start of the contents and can be moved past the end,
        // which a later read answers with nothing rather than an error. What cannot be expressed is
        // a position before the start, or one that does not fit the arithmetic at all.
        let position = match pos {
            SeekFrom::Start(offset) => as_position(offset)?,
            SeekFrom::Current(delta) => as_position(self.pos)?
                .checked_add(delta)
                .ok_or(DeviceError::InvalidPath)?,
            SeekFrom::End(delta) => as_position(self.size)?
                .checked_add(delta)
                .ok_or(DeviceError::InvalidPath)?,
        };

        if position < 0 {
            return Err(DeviceError::InvalidPath);
        }

        // Non-negative by the guard above.
        self.pos = position as u64;
        Ok(self.pos)
    }

    fn metadata(&self) -> Result<Metadata, DeviceError> {
        Ok(Metadata::file(self.size))
    }
}

/// Returns `offset` as the signed position the arithmetic above works in.
///
/// # Errors
///
/// Returns [`DeviceError::InvalidPath`] when the position is past what can be expressed, which for
/// a file that fits an image cannot be reached by reading and only by seeking there deliberately.
fn as_position(offset: u64) -> Result<i64, DeviceError> {
    i64::try_from(offset).map_err(|_| DeviceError::InvalidPath)
}
