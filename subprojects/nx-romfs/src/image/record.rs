//! The shapes a romfs image is written in.
//!
//! Every field is little-endian regardless of the machine reading it, so each one is declared as a
//! byte-order type rather than a plain integer: the declaration is the decoder, and no call site
//! has to remember to swap.
//!
//! A record is followed immediately by its name, which is why none of these types carries the name
//! itself. The name's length is in the record, and reading the bytes after it belongs to whoever
//! holds the table, because only the table knows where the record ends.

use zerocopy::little_endian::{
    U32,
    U64,
};

/// Offset that names no entry, used to end a sibling or hash chain.
pub const NONE: u32 = u32::MAX;

/// The fixed header at the start of an image.
///
/// Every offset in it is measured from the start of the image, and every size is in bytes.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    zerocopy::Unaligned,
)]
#[repr(C)]
pub struct Header {
    /// Size of this header.
    pub header_size: U64,
    /// Where the directory hash buckets start.
    pub dir_hash_table_off: U64,
    /// How many bytes of directory hash buckets there are.
    pub dir_hash_table_size: U64,
    /// Where the directory records start.
    pub dir_table_off: U64,
    /// How many bytes of directory records there are.
    pub dir_table_size: U64,
    /// Where the file hash buckets start.
    pub file_hash_table_off: U64,
    /// How many bytes of file hash buckets there are.
    pub file_hash_table_size: U64,
    /// Where the file records start.
    pub file_table_off: U64,
    /// How many bytes of file records there are.
    pub file_table_size: U64,
    /// Where the file contents start.
    pub file_data_off: U64,
}

/// One directory, at some offset into the directory table.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    zerocopy::Unaligned,
)]
#[repr(C)]
pub struct DirRecord {
    /// Offset of the directory holding this one. The root names itself.
    pub parent: U32,
    /// Offset of the next directory with the same parent, or [`NONE`].
    pub sibling: U32,
    /// Offset of the first directory inside this one, or [`NONE`].
    pub child_dir: U32,
    /// Offset of the first file inside this one, or [`NONE`].
    pub child_file: U32,
    /// Offset of the next directory in the same hash bucket, or [`NONE`].
    pub next_hash: U32,
    /// Length of the name that follows this record, in bytes.
    pub name_len: U32,
}

/// One file, at some offset into the file table.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
    zerocopy::Unaligned,
)]
#[repr(C)]
pub struct FileRecord {
    /// Offset of the directory holding this file.
    pub parent: U32,
    /// Offset of the next file with the same parent, or [`NONE`].
    pub sibling: U32,
    /// Where this file's contents start, measured from the image's file data.
    pub data_off: U64,
    /// How many bytes of contents this file has.
    pub data_size: U64,
    /// Offset of the next file in the same hash bucket, or [`NONE`].
    pub next_hash: U32,
    /// Length of the name that follows this record, in bytes.
    pub name_len: U32,
}

/// Returns which of `buckets` hash buckets holds the entry called `name` inside `parent`.
///
/// The parent's offset is mixed in, which is what lets two files of the same name in different
/// directories land in different buckets. The constant and the rotation are the image format's;
/// changing either stops finding entries the builder placed.
pub fn bucket_of(parent: u32, name: &[u8], buckets: u32) -> u32 {
    let mut hash = parent ^ 123_456_789;
    for byte in name {
        hash = hash.rotate_right(5);
        hash ^= u32::from(*byte);
    }
    hash % buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_of_same_name_under_different_parents_gives_different_buckets() {
        //* Given
        let buckets = 64;

        //* When
        let under_root = bucket_of(0, b"config.json", buckets);
        let under_other = bucket_of(0x20, b"config.json", buckets);

        //* Then
        assert_ne!(
            under_root, under_other,
            "the parent has to reach the hash, or every directory would share one chain"
        );
    }

    #[test]
    fn bucket_of_any_name_stays_inside_the_table() {
        //* Given
        let buckets = 7;

        //* When / Then
        for name in [b"a".as_slice(), b"bb", b"ccc", b"a rather longer name"] {
            let bucket = bucket_of(0, name, buckets);
            assert!(
                bucket < buckets,
                "a bucket outside the table would index past the end of it"
            );
        }
    }

    #[test]
    fn bucket_of_empty_name_leaves_the_seed_untouched() {
        //* Given / When
        let bucket = bucket_of(0, b"", 16);

        //* Then
        assert_eq!(
            bucket,
            (0u32 ^ 123_456_789) % 16,
            "an empty name leaves the seed untouched"
        );
    }
}
