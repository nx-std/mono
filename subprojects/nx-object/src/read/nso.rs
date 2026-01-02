use zerocopy::FromBytes;

use crate::raw::nso::{NSO_MAGIC, NsoFlags, NsoHeader};

/// High-level NSO parser with compressed segment access.
pub struct Nso<'a> {
    bytes: &'a [u8],
    header: &'a NsoHeader,
}

impl<'a> Nso<'a> {
    /// Parse NSO from bytes with magic and size validation.
    pub fn try_from_bytes(bytes: &'a [u8]) -> Result<Self, FromBytesError> {
        if bytes.len() < size_of::<NsoHeader>() {
            return Err(FromBytesError::BufferTooSmall {
                required: size_of::<NsoHeader>(),
                available: bytes.len(),
            });
        }

        let header = NsoHeader::ref_from_prefix(bytes)
            .map_err(|_| FromBytesError::BufferTooSmall {
                required: 0x100,
                available: bytes.len(),
            })?
            .0;

        if header.magic.get() != NSO_MAGIC {
            return Err(FromBytesError::InvalidMagic {
                found: header.magic.get(),
            });
        }

        Ok(Self { bytes, header })
    }

    /// Create from raw pointer
    ///
    /// # Safety
    /// - `ptr` must point to valid NSO data
    /// - The memory must remain valid for lifetime `'a`
    pub unsafe fn try_from_ptr(ptr: *const u8) -> Result<Self, FromPtrError> {
        // SAFETY: Caller guarantees ptr is valid and memory remains valid for 'a
        let bytes = unsafe { core::slice::from_raw_parts(ptr, usize::MAX / 2) };
        Self::try_from_bytes(bytes).map_err(FromPtrError)
    }

    /// Get the NSO header.
    pub fn header(&self) -> &NsoHeader {
        self.header
    }

    /// Get the 32-byte module ID.
    pub fn module_id(&self) -> &[u8; 32] {
        &self.header.module_id
    }

    /// Get the NSO flags.
    pub fn flags(&self) -> NsoFlags {
        NsoFlags::from_bits_truncate(self.header.flags.get())
    }

    /// Get compressed text segment bytes (raw, not decompressed)
    pub fn text_compressed(&self) -> &'a [u8] {
        let off = self.header.text.file_offset.get() as usize;
        let size = self.header.text_file_size.get() as usize;
        &self.bytes[off..off + size]
    }

    /// Get compressed rodata segment bytes
    pub fn rodata_compressed(&self) -> &'a [u8] {
        let off = self.header.rodata.file_offset.get() as usize;
        let size = self.header.rodata_file_size.get() as usize;
        &self.bytes[off..off + size]
    }

    /// Get compressed data segment bytes
    pub fn data_compressed(&self) -> &'a [u8] {
        let off = self.header.data.file_offset.get() as usize;
        let size = self.header.data_file_size.get() as usize;
        &self.bytes[off..off + size]
    }
}

/// Errors that can occur when parsing an NSO from bytes
#[derive(Debug, thiserror::Error)]
pub enum FromBytesError {
    /// Buffer is too small to contain the required data
    #[error("buffer too small: need {required} bytes, have {available}")]
    BufferTooSmall {
        /// Number of bytes required
        required: usize,
        /// Number of bytes available
        available: usize,
    },
    /// Magic number does not match NSO0 (0x304f534e)
    #[error("invalid magic: expected 0x304f534e (NSO0), found {found:#010x}")]
    InvalidMagic {
        /// Found magic number
        found: u32,
    },
}

/// Error when parsing NSO from raw pointer
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FromPtrError(FromBytesError);
