//! A mounted image's four tables, and everything answered out of them.
//!
//! Mounting reads the header and all four tables into memory once. From then on a path lookup
//! touches nothing but this: the tree is 32-bit offsets into the two record tables, and finding an
//! entry by name is a hash into a bucket followed by a walk down one chain.
//!
//! ## Every offset is checked before it is followed
//!
//! The tables come off storage a program does not control, so an offset read out of one is not
//! evidence that anything is there. [`Image::dir_at`] and [`Image::file_at`] are the only two
//! places a raw offset turns into a record, and both refuse an offset whose record, or whose
//! record's name, would run past the end of the table holding it.
//!
//! Chain walks are bounded for the same reason. A `hash_sibling_offset` or `sibling_offset` field
//! pointing back up its own chain is a loop, and an image can be written that way; each walk
//! therefore gives up after more steps than the table could hold records.
//!
//! ## The layouts are not declared here
//!
//! [`nx_object`] declares them, and the image writer this workspace builds with reads them out of
//! the same place. The hash in particular has to agree byte for byte with whatever laid the buckets
//! out, so it is one definition rather than one per side.

use alloc::{
    vec,
    vec::Vec,
};

use nx_object::raw::romfs::{
    NO_ENTRY,
    RomFsDirEntry,
    RomFsFileEntry,
    RomFsHeader,
    path_hash,
};
use nx_sys_fd::device::DeviceError;
use zerocopy::FromBytes as _;

use crate::source::Source;

/// Offset of the root directory, which is always the first record in the directory table.
pub(crate) const ROOT: u32 = 0;

/// Largest table this crate will read into memory, in bytes.
///
/// The header is untrusted, and its sizes are what a mount allocates from. A real image's tables
/// are kilobytes; this bound exists so that a header claiming gigabytes is refused with an error
/// rather than answered with an allocation.
const MAX_TABLE_LEN: u64 = 64 * 1024 * 1024;

/// One mounted image: its tables, and the bytes they came out of.
///
/// The source belongs here rather than beside it because the tables and the file contents are two
/// halves of one thing. A caller that has an image has everything needed to answer about it, and
/// nothing above this module has to know that reading a file costs a command while reading a
/// directory does not.
pub(crate) struct Image {
    /// Where the image is read from, for the file contents the tables do not hold.
    source: Source,
    /// Hash buckets chaining into [`Self::dir_table`], four bytes each.
    dir_hash: Vec<u8>,
    /// The directory records, addressed by byte offset.
    dir_table: Vec<u8>,
    /// Hash buckets chaining into [`Self::file_table`], four bytes each.
    file_hash: Vec<u8>,
    /// The file records, addressed by byte offset.
    file_table: Vec<u8>,
    /// Where file contents start, measured from the start of the image.
    file_data_off: u64,
}

impl Image {
    /// Reads the header and every table out of `source`, and takes it over.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::Io`] when the image could not be read, [`LoadError::TableTooLarge`]
    /// when the header asks for more memory than a table may take, [`LoadError::NoBuckets`] when a
    /// hash table has no buckets to hash into, and [`LoadError::NoRoot`] when the directory table
    /// is too small to hold the root.
    pub(crate) fn load(source: Source) -> Result<Self, LoadError> {
        let mut header_bytes = [0u8; size_of::<RomFsHeader>()];
        source
            .read_exact_at(0, &mut header_bytes)
            .map_err(|_| LoadError::Io)?;
        // The buffer is exactly the header's size and the header is all byte-order fields, so the
        // only way this fails is a length mismatch that cannot happen here.
        let Ok(header) = RomFsHeader::read_from_bytes(&header_bytes) else {
            return Err(LoadError::Io);
        };

        let dir_hash = read_table(
            &source,
            header.dir_hash_table_offset.get(),
            header.dir_hash_table_size.get(),
        )?;
        let dir_table = read_table(
            &source,
            header.dir_meta_table_offset.get(),
            header.dir_meta_table_size.get(),
        )?;
        let file_hash = read_table(
            &source,
            header.file_hash_table_offset.get(),
            header.file_hash_table_size.get(),
        )?;
        let file_table = read_table(
            &source,
            header.file_meta_table_offset.get(),
            header.file_meta_table_size.get(),
        )?;

        // A bucket count of zero would divide by zero on the first lookup, and a directory table
        // too short to hold the root leaves nothing for a path to start from.
        if dir_hash.len() < 4 || file_hash.len() < 4 {
            return Err(LoadError::NoBuckets);
        }
        if dir_table.len() < size_of::<RomFsDirEntry>() {
            return Err(LoadError::NoRoot);
        }

        Ok(Self {
            source,
            dir_hash,
            dir_table,
            file_hash,
            file_table,
            file_data_off: header.file_data_offset.get(),
        })
    }

    /// Returns where the contents of the file at `off` start, and how many bytes there are.
    ///
    /// The offset returned is what [`Image::read_contents`] takes; it is absolute within the
    /// container, so nothing above has to know where the image itself starts.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off` names no file.
    pub(crate) fn contents_of(&self, off: u32) -> Result<(u64, u64), DeviceError> {
        let (file, _) = self.file_at(off).ok_or(DeviceError::Io)?;
        Ok((
            self.file_data_off + file.data_offset.get(),
            file.data_size.get(),
        ))
    }

    /// Fills `buf` from `offset`, which [`Image::contents_of`] produced.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the bytes could not be read.
    pub(crate) fn read_contents(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        self.source.read_exact_at(offset, buf)
    }

    /// Returns how large the file at `off` is.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off` names no file.
    pub(crate) fn file_size_at(&self, off: u32) -> Result<u64, DeviceError> {
        let (file, _) = self.file_at(off).ok_or(DeviceError::Io)?;
        Ok(file.data_size.get())
    }

    /// Returns the name of the directory at `off`, and where its siblings continue.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off` names no directory.
    pub(crate) fn dir_entry_at(&self, off: u32) -> Result<(&[u8], Option<u32>), DeviceError> {
        let (dir, name) = self.dir_at(off).ok_or(DeviceError::Io)?;
        Ok((name, chained(dir.sibling_offset.get())))
    }

    /// Returns the name and size of the file at `off`, and where its siblings continue.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off` names no file.
    pub(crate) fn file_entry_at(&self, off: u32) -> Result<(&[u8], u64, Option<u32>), DeviceError> {
        let (file, name) = self.file_at(off).ok_or(DeviceError::Io)?;
        Ok((
            name,
            file.data_size.get(),
            chained(file.sibling_offset.get()),
        ))
    }

    /// Returns where the first child directory and the first child file of `off` are.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off` names no directory.
    pub(crate) fn children_of(&self, off: u32) -> Result<(Option<u32>, Option<u32>), DeviceError> {
        let (dir, _) = self.dir_at(off).ok_or(DeviceError::Io)?;
        Ok((
            chained(dir.child_offset.get()),
            chained(dir.file_offset.get()),
        ))
    }

    /// Resolves `path` to the directory holding what it names, and whatever component is left over.
    ///
    /// With `consume_last` the whole path is walked as directories and the leftover is empty; that
    /// is what a directory operation wants. Without it the final component is left for the caller
    /// to look up as a file, which is what opening one wants.
    ///
    /// Relative paths start at `cwd`; a path beginning with `/` starts at the root.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::InvalidPath`] when the path is empty or holds an empty component,
    /// [`DeviceError::NotFound`] when a component names no directory, and [`DeviceError::Io`] when
    /// a record the walk followed is not where the image said it was.
    pub(crate) fn walk<'p>(
        &self,
        cwd: u32,
        path: &'p [u8],
        consume_last: bool,
    ) -> Result<(u32, &'p [u8]), DeviceError> {
        if path.is_empty() {
            return Err(DeviceError::InvalidPath);
        }

        let mut dir = cwd;
        let mut rest = path;
        if rest[0] == b'/' {
            dir = ROOT;
            rest = &rest[1..];
        }

        while !rest.is_empty() {
            let component = match rest.iter().position(|byte| *byte == b'/') {
                // A path such as `a//b` names nothing, and treating the empty component as `.`
                // would quietly accept it.
                Some(0) => return Err(DeviceError::InvalidPath),
                Some(slash) => {
                    let component = &rest[..slash];
                    rest = &rest[slash + 1..];
                    component
                }
                None if consume_last => {
                    let component = rest;
                    rest = &[];
                    component
                }
                // The last component is the caller's to look up, so the walk stops here with the
                // directory that should hold it.
                None => return Ok((dir, rest)),
            };

            dir = match component {
                b"." => continue,
                b".." => self.parent_of(dir)?,
                _ => self
                    .find_dir(dir, component)?
                    .ok_or(DeviceError::NotFound)?,
            };
        }

        Ok((dir, rest))
    }

    /// Returns where the directory called `name` inside `parent` is, if it is there.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the chain leads somewhere the table does not reach.
    pub(crate) fn find_dir(&self, parent: u32, name: &[u8]) -> Result<Option<u32>, DeviceError> {
        let buckets = (self.dir_hash.len() / 4) as u32;
        let bucket = bucket_of(parent, name, buckets);
        let mut off = self.bucket(&self.dir_hash, bucket)?;

        for _ in 0..self.max_dir_records() {
            if off == NO_ENTRY {
                return Ok(None);
            }

            let (dir, entry_name) = self.dir_at(off).ok_or(DeviceError::Io)?;
            if dir.parent_offset.get() == parent && entry_name == name {
                return Ok(Some(off));
            }
            off = dir.hash_sibling_offset.get();
        }

        // More steps than the table has records means the chain leads back into itself.
        Err(DeviceError::Io)
    }

    /// Returns where the file called `name` inside `parent` is, if it is there.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the chain leads somewhere the table does not reach.
    pub(crate) fn find_file(&self, parent: u32, name: &[u8]) -> Result<Option<u32>, DeviceError> {
        let buckets = (self.file_hash.len() / 4) as u32;
        let bucket = bucket_of(parent, name, buckets);
        let mut off = self.bucket(&self.file_hash, bucket)?;

        for _ in 0..self.max_file_records() {
            if off == NO_ENTRY {
                return Ok(None);
            }

            let (file, entry_name) = self.file_at(off).ok_or(DeviceError::Io)?;
            if file.parent_offset.get() == parent && entry_name == name {
                return Ok(Some(off));
            }
            off = file.hash_sibling_offset.get();
        }

        Err(DeviceError::Io)
    }

    /// Returns where the directory holding `off` is.
    ///
    /// The root names itself, so `..` at the top stays at the top, which is what every filesystem
    /// does.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `off`, or the parent it names, is not in the table.
    fn parent_of(&self, off: u32) -> Result<u32, DeviceError> {
        let (dir, _) = self.dir_at(off).ok_or(DeviceError::Io)?;
        let parent = dir.parent_offset.get();

        if self.dir_at(parent).is_none() {
            return Err(DeviceError::Io);
        }
        Ok(parent)
    }

    /// Returns the directory record at `off` and the name that follows it.
    ///
    /// `None` means the offset names nothing: either the record itself or the name after it would
    /// run past the end of the table.
    fn dir_at(&self, off: u32) -> Option<(&RomFsDirEntry, &[u8])> {
        let bytes = self.dir_table.get(off as usize..)?;
        let (dir, after) = RomFsDirEntry::ref_from_prefix(bytes).ok()?;
        let name = after.get(..dir.name_len.get() as usize)?;
        Some((dir, name))
    }

    /// Returns the file record at `off` and the name that follows it. See [`Image::dir_at`].
    fn file_at(&self, off: u32) -> Option<(&RomFsFileEntry, &[u8])> {
        let bytes = self.file_table.get(off as usize..)?;
        let (file, after) = RomFsFileEntry::ref_from_prefix(bytes).ok()?;
        let name = after.get(..file.name_len.get() as usize)?;
        Some((file, name))
    }

    /// Returns what bucket `index` of `table` chains to.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the bucket is outside the table, which the hash makes
    /// impossible and is therefore a table that changed under the lookup.
    fn bucket(&self, table: &[u8], index: u32) -> Result<u32, DeviceError> {
        let start = index as usize * 4;
        let bytes = table.get(start..start + 4).ok_or(DeviceError::Io)?;
        // The slice is exactly four bytes, which is the whole of the type.
        let value =
            zerocopy::little_endian::U32::read_from_bytes(bytes).map_err(|_| DeviceError::Io)?;
        Ok(value.get())
    }

    /// Returns more steps than any chain through the directory table can take.
    fn max_dir_records(&self) -> usize {
        self.dir_table.len() / size_of::<RomFsDirEntry>() + 1
    }

    /// Returns more steps than any chain through the file table can take.
    fn max_file_records(&self) -> usize {
        self.file_table.len() / size_of::<RomFsFileEntry>() + 1
    }
}

/// Errors returned by [`Image::load`].
///
/// Reachable outside the crate through [`crate::mount`], which re-exports it: a caller that failed
/// to mount an image has to be able to name why.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The image could not be read
    ///
    /// Occurs when the container ended early or the read failed. Nothing was mounted.
    #[error("failed to read the image")]
    Io,

    /// A table is larger than one may be
    ///
    /// Occurs when the header claims a table this crate will not allocate for. The image is either
    /// corrupt or is not a romfs image at all.
    #[error("the image claims a table of {0} bytes")]
    TableTooLarge(u64),

    /// A hash table holds no buckets
    ///
    /// Occurs when the header gives a hash table fewer than four bytes, leaving nothing to hash
    /// into. Every lookup on such an image would divide by zero.
    #[error("the image has a hash table with no buckets")]
    NoBuckets,

    /// The directory table cannot hold the root
    ///
    /// Occurs when the header gives the directory table fewer bytes than one record, so a path has
    /// nowhere to start.
    #[error("the image has no root directory")]
    NoRoot,
}

/// Returns what `off` chains to, or nothing when the chain ends there.
///
/// The image marks the end of a chain with a reserved offset. Turning it into `None` here is what
/// keeps that sentinel from travelling: a caller walking siblings matches on an `Option` and cannot
/// mistake the end of the chain for an entry.
fn chained(off: u32) -> Option<u32> {
    (off != NO_ENTRY).then_some(off)
}

/// Returns which of `buckets` hash buckets holds the entry called `name` inside `parent`.
///
/// The hash itself belongs to the format and is shared with whatever wrote the image. Reducing it
/// to a bucket is this crate's part, because the count comes from a table only this module holds.
fn bucket_of(parent: u32, name: &[u8], buckets: u32) -> u32 {
    // `buckets` is a quarter of a hash table's length, and [`Image::load`] refuses an image whose
    // hash table is shorter than one bucket, so the divisor is never zero by the time a lookup runs.
    path_hash(parent, name) % buckets
}

#[cfg(test)]
mod tests {
    use super::{
        ROOT,
        bucket_of,
    };

    #[test]
    fn bucket_of_a_name_longer_than_the_table_stays_inside_it() {
        //* Given
        // Fewer buckets than the name has bytes, so an unreduced hash lands far outside them.
        let buckets = 7;
        let name = b"a rather longer name";

        //* When
        let bucket = bucket_of(ROOT, name, buckets);

        //* Then
        assert!(
            bucket < buckets,
            "a bucket outside the table would index past the end of it"
        );
    }
}

/// Reads `len` bytes from `off` into a table of its own.
///
/// # Errors
///
/// Returns [`LoadError::TableTooLarge`] when the header asks for more than a table may take, and
/// [`LoadError::Io`] when the bytes could not be read.
fn read_table(source: &Source, off: u64, len: u64) -> Result<Vec<u8>, LoadError> {
    if len > MAX_TABLE_LEN {
        return Err(LoadError::TableTooLarge(len));
    }

    // Bounded by `MAX_TABLE_LEN` above, which is well inside a `usize` on this target.
    let mut table = vec![0u8; len as usize];
    source
        .read_exact_at(off, &mut table)
        .map_err(|_| LoadError::Io)?;

    Ok(table)
}
