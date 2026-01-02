//! NSO (Nintendo Software Object) builder.

use std::vec::Vec;

use zerocopy::FromZeros;

use crate::raw::{
    build_id::BuildId,
    nso::{NSO_MAGIC, NsoFlags, NsoHeader, NsoSegmentHeader},
};

/// Builder for constructing NSO files.
pub struct NsoBuilder {
    text: Option<Vec<u8>>,
    rodata: Option<Vec<u8>>,
    data: Option<Vec<u8>>,
    bss_size: u32,
    module_id: Option<BuildId>,
    #[cfg(feature = "lz4")]
    compress: bool,
}

impl NsoBuilder {
    /// Create a new NSO builder.
    ///
    /// Compression is enabled by default when the `lz4` feature is active.
    pub fn new() -> Self {
        Self {
            text: None,
            rodata: None,
            data: None,
            bss_size: 0,
            module_id: None,
            #[cfg(feature = "lz4")]
            compress: true,
        }
    }

    /// Set the text (code) segment.
    pub fn text(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.text = Some(data.into());
        self
    }

    /// Set the rodata (read-only data) segment.
    pub fn rodata(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.rodata = Some(data.into());
        self
    }

    /// Set the data (read-write data) segment.
    pub fn data(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Set the BSS section size in bytes.
    pub fn bss_size(mut self, size: u32) -> Self {
        self.bss_size = size;
        self
    }

    /// Set the 32-byte module ID (build ID).
    ///
    /// If not provided, will default to all zeros.
    pub fn module_id(mut self, id: BuildId) -> Self {
        self.module_id = Some(id);
        self
    }

    /// Enable or disable LZ4 compression of segments.
    ///
    /// Compression is enabled by default when the `lz4` feature is active.
    /// This method is only available when the `lz4` feature is enabled.
    #[cfg(feature = "lz4")]
    pub fn compressed(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    /// Build the complete NSO file.
    pub fn build(self) -> Result<Vec<u8>, BuildError> {
        // Validate required fields
        let text = self.text.ok_or(BuildError::MissingText)?;
        let rodata = self.rodata.ok_or(BuildError::MissingRodata)?;
        let data = self.data.ok_or(BuildError::MissingData)?;

        // Pad segments to 0x1000 alignment
        let text_padded = pad_to_alignment(&text, 0x1000);
        let rodata_padded = pad_to_alignment(&rodata, 0x1000);
        let data_padded = pad_to_alignment(&data, 0x1000);

        // Compute SHA256 hashes of uncompressed segments
        let text_hash = sha256(&text_padded);
        let rodata_hash = sha256(&rodata_padded);
        let data_hash = sha256(&data_padded);

        // Compress segments if enabled
        #[cfg(feature = "lz4")]
        let (
            text_data,
            text_compressed,
            rodata_data,
            rodata_compressed,
            data_data,
            data_compressed,
        ) = if self.compress {
            (
                lz4_compress(&text_padded)?,
                true,
                lz4_compress(&rodata_padded)?,
                true,
                lz4_compress(&data_padded)?,
                true,
            )
        } else {
            (
                text_padded.clone(),
                false,
                rodata_padded.clone(),
                false,
                data_padded.clone(),
                false,
            )
        };

        #[cfg(not(feature = "lz4"))]
        let (
            text_data,
            text_compressed,
            rodata_data,
            rodata_compressed,
            data_data,
            data_compressed,
        ) = {
            (
                text_padded.clone(),
                false,
                rodata_padded.clone(),
                false,
                data_padded.clone(),
                false,
            )
        };

        // Build flags
        let mut flags = NsoFlags::empty();
        if text_compressed {
            flags |= NsoFlags::TEXT_COMPRESS;
        }
        if rodata_compressed {
            flags |= NsoFlags::RODATA_COMPRESS;
        }
        if data_compressed {
            flags |= NsoFlags::DATA_COMPRESS;
        }
        // Always include hash flags
        flags |= NsoFlags::TEXT_HASH | NsoFlags::RODATA_HASH | NsoFlags::DATA_HASH;

        // Calculate file offsets (segments start after 0x100 header)
        let text_offset = 0x100u32;
        let rodata_offset = text_offset + text_data.len() as u32;
        let data_offset = rodata_offset + rodata_data.len() as u32;

        // Calculate memory offsets (segments are loaded contiguously in memory)
        let text_mem_offset = 0u32;
        let rodata_mem_offset = text_mem_offset + text_padded.len() as u32;
        let data_mem_offset = rodata_mem_offset + rodata_padded.len() as u32;

        // Build header
        let mut header = NsoHeader::new_zeroed();
        header.magic = NSO_MAGIC.into();
        header.version = 0.into();
        header.flags = flags.bits().into();
        header.text = NsoSegmentHeader {
            file_offset: text_offset.into(),
            memory_offset: text_mem_offset.into(),
            size: (text_padded.len() as u32).into(),
        };
        header.module_name_offset = 0.into();
        header.rodata = NsoSegmentHeader {
            file_offset: rodata_offset.into(),
            memory_offset: rodata_mem_offset.into(),
            size: (rodata_padded.len() as u32).into(),
        };
        header.module_name_size = 0.into();
        header.data = NsoSegmentHeader {
            file_offset: data_offset.into(),
            memory_offset: data_mem_offset.into(),
            size: (data_padded.len() as u32).into(),
        };
        header.bss_size = self.bss_size.into();
        header.module_id = self.module_id.unwrap_or([0u8; 0x20]);
        header.text_file_size = (text_data.len() as u32).into();
        header.rodata_file_size = (rodata_data.len() as u32).into();
        header.data_file_size = (data_data.len() as u32).into();
        header.embedded_offset = 0.into();
        header.embedded_size = 0.into();
        header.dynstr_offset = 0.into();
        header.dynstr_size = 0.into();
        header.dynsym_offset = 0.into();
        header.dynsym_size = 0.into();
        header.text_hash = text_hash;
        header.rodata_hash = rodata_hash;
        header.data_hash = data_hash;

        // Build output buffer
        let total_size = 0x100 + text_data.len() + rodata_data.len() + data_data.len();
        let mut buf = Vec::with_capacity(total_size);

        // Write header
        buf.extend_from_slice(zerocopy::IntoBytes::as_bytes(&header));

        // Write segments
        buf.extend_from_slice(&text_data);
        buf.extend_from_slice(&rodata_data);
        buf.extend_from_slice(&data_data);

        Ok(buf)
    }
}

impl Default for NsoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned by [`NsoBuilder::build`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Text segment was not provided.
    #[error("missing text segment")]
    MissingText,
    /// Rodata segment was not provided.
    #[error("missing rodata segment")]
    MissingRodata,
    /// Data segment was not provided.
    #[error("missing data segment")]
    MissingData,
    /// LZ4 compression failed.
    #[cfg(feature = "lz4")]
    #[error("lz4 compression failed")]
    CompressionFailed,
}

/// Pad a byte slice to the specified alignment.
fn pad_to_alignment(data: &[u8], alignment: usize) -> Vec<u8> {
    let len = data.len();
    let padded_len = (len + alignment - 1) / alignment * alignment;
    let mut padded = Vec::with_capacity(padded_len);
    padded.extend_from_slice(data);
    padded.resize(padded_len, 0);
    padded
}

/// Compute SHA256 hash of data.
fn sha256(data: &[u8]) -> [u8; 0x20] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compress data with LZ4.
#[cfg(feature = "lz4")]
fn lz4_compress(data: &[u8]) -> Result<Vec<u8>, BuildError> {
    lz4_flex::compress_prepend_size(data);
    Ok(lz4_flex::compress_prepend_size(data))
}
