use zerocopy::FromBytes;

use crate::raw::nro::{
    ASSET_MAGIC, NRO_MAGIC, NroAssetHeader, NroAssetSection, NroHeader, NroStart,
};

/// High-level NRO parser with segment and asset access.
pub struct Nro<'a> {
    bytes: &'a [u8],
    start: &'a NroStart,
    header: &'a NroHeader,
    asset_header: Option<&'a NroAssetHeader>,
}

impl<'a> Nro<'a> {
    /// Parse NRO from bytes with magic and size validation.
    pub fn try_from_bytes(bytes: &'a [u8]) -> Result<Self, FromBytesError> {
        // Validate minimum size for start + header
        let min_size = size_of::<NroStart>() + size_of::<NroHeader>();
        if bytes.len() < min_size {
            return Err(FromBytesError::BufferTooSmall {
                required: min_size,
                available: bytes.len(),
            });
        }

        let start = NroStart::ref_from_prefix(bytes)
            .map_err(|_| FromBytesError::BufferTooSmall {
                required: 0x10,
                available: bytes.len(),
            })?
            .0;

        let header = NroHeader::ref_from_prefix(&bytes[0x10..])
            .map_err(|_| FromBytesError::BufferTooSmall {
                required: 0x80,
                available: bytes.len(),
            })?
            .0;

        // Validate magic
        if header.magic.get() != NRO_MAGIC {
            return Err(FromBytesError::InvalidMagic {
                found: header.magic.get(),
            });
        }

        // Try to parse asset header (at end of NRO)
        let nro_size = header.size.get() as usize;
        let asset_header = if bytes.len() > nro_size + size_of::<NroAssetHeader>() {
            NroAssetHeader::ref_from_prefix(&bytes[nro_size..])
                .ok()
                .map(|(h, _)| h)
                .filter(|h| h.magic.get() == ASSET_MAGIC)
        } else {
            None
        };

        Ok(Self {
            bytes,
            start,
            header,
            asset_header,
        })
    }

    /// Get the NRO start structure.
    pub fn start(&self) -> &NroStart {
        self.start
    }

    /// Get the NRO header.
    pub fn header(&self) -> &NroHeader {
        self.header
    }

    /// Get the asset header if present.
    pub fn asset_header(&self) -> Option<&NroAssetHeader> {
        self.asset_header
    }

    /// Get the 32-byte build ID.
    pub fn build_id(&self) -> &[u8; 32] {
        &self.header.build_id
    }

    /// Get the text (code) segment bytes.
    pub fn text_segment(&self) -> &[u8] {
        self.segment(0)
    }

    /// Get the read-only data segment bytes.
    pub fn rodata_segment(&self) -> &[u8] {
        self.segment(1)
    }

    /// Get the data segment bytes.
    pub fn data_segment(&self) -> &[u8] {
        self.segment(2)
    }

    fn segment(&self, idx: usize) -> &[u8] {
        let seg = &self.header.segments[idx];
        let off = seg.file_off.get() as usize;
        let size = seg.size.get() as usize;
        &self.bytes[off..off + size]
    }

    /// Get the icon asset bytes if present.
    pub fn asset_icon(&self) -> Option<&'a [u8]> {
        self.asset_section(|h| &h.icon)
    }

    /// Get the NACP asset bytes if present.
    pub fn asset_nacp(&self) -> Option<&'a [u8]> {
        self.asset_section(|h| &h.nacp)
    }

    /// Get the RomFS asset bytes if present.
    pub fn asset_romfs(&self) -> Option<&'a [u8]> {
        self.asset_section(|h| &h.romfs)
    }

    fn asset_section<F>(&self, f: F) -> Option<&'a [u8]>
    where
        F: FnOnce(&NroAssetHeader) -> &NroAssetSection,
    {
        let header = self.asset_header?;
        let section = f(header);
        let base = self.header.size.get() as usize;
        let off = base + section.offset.get() as usize;
        let size = section.size.get() as usize;
        if size == 0 {
            return None;
        }
        Some(&self.bytes[off..off + size])
    }

    /// Create from raw pointer (for runtime introspection of loaded module)
    ///
    /// # Safety
    /// - `ptr` must point to valid NRO data
    /// - The memory must remain valid for lifetime `'a`
    pub unsafe fn try_from_ptr(ptr: *const u8) -> Result<Self, FromPtrError> {
        // Create slice from pointer - we don't know size yet, use max reasonable
        // SAFETY: Caller guarantees ptr is valid and memory remains valid for 'a
        let bytes = unsafe { core::slice::from_raw_parts(ptr, usize::MAX / 2) };
        Self::try_from_bytes(bytes).map_err(FromPtrError)
    }
}

/// Errors that can occur when parsing an NRO from bytes
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
    /// Magic number does not match NRO0 (0x304f524e)
    #[error("invalid magic: expected 0x304f524e (NRO0), found {found:#010x}")]
    InvalidMagic {
        /// Found magic number
        found: u32,
    },
}

/// Error when parsing NRO from raw pointer
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FromPtrError(FromBytesError);
