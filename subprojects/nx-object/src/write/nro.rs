//! NRO (Nintendo Relocatable Object) builder.

use std::vec::Vec;

use zerocopy::FromZeros;

use crate::raw::{
    build_id::BuildId,
    nro::{ASSET_MAGIC, NRO_MAGIC, NroAssetHeader, NroHeader, NroSegment, NroStart},
};

/// Builder for constructing NRO files.
pub struct NroBuilder {
    text: Option<Vec<u8>>,
    rodata: Option<Vec<u8>>,
    data: Option<Vec<u8>>,
    bss_size: u32,
    build_id: Option<BuildId>,
    mod0_offset: Option<u32>,
    icon: Option<Vec<u8>>,
    nacp: Option<Vec<u8>>,
    romfs: Option<Vec<u8>>,
}

impl NroBuilder {
    /// Create a new NRO builder.
    pub fn new() -> Self {
        Self {
            text: None,
            rodata: None,
            data: None,
            bss_size: 0,
            build_id: None,
            mod0_offset: None,
            icon: None,
            nacp: None,
            romfs: None,
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

    /// Set the 32-byte build ID.
    ///
    /// If not provided, will default to all zeros.
    pub fn build_id(mut self, id: BuildId) -> Self {
        self.build_id = Some(id);
        self
    }

    /// Set the MOD0 offset (relative to NRO start).
    ///
    /// If not provided, defaults to 0.
    pub fn mod0_offset(mut self, offset: u32) -> Self {
        self.mod0_offset = Some(offset);
        self
    }

    /// Add an icon asset (JPEG image).
    pub fn asset_icon(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.icon = Some(data.into());
        self
    }

    /// Add a NACP (Nintendo Application Control Property) asset.
    pub fn asset_nacp(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.nacp = Some(data.into());
        self
    }

    /// Add a RomFS asset.
    pub fn asset_romfs(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.romfs = Some(data.into());
        self
    }

    /// Build the complete NRO file.
    pub fn build(self) -> Result<Vec<u8>, BuildError> {
        // Validate required fields
        let text = self.text.ok_or(BuildError::MissingText)?;
        let rodata = self.rodata.ok_or(BuildError::MissingRodata)?;
        let data = self.data.ok_or(BuildError::MissingData)?;

        // Pad segments to 0x1000 alignment
        let text_padded = pad_to_alignment(&text, 0x1000);
        let rodata_padded = pad_to_alignment(&rodata, 0x1000);
        let data_padded = pad_to_alignment(&data, 0x1000);

        // Calculate segment offsets
        // Segments start after NroStart (0x10) + NroHeader (0x70) = 0x80
        let text_offset = 0x80u32;
        let rodata_offset = text_offset + text_padded.len() as u32;
        let data_offset = rodata_offset + rodata_padded.len() as u32;
        let nro_size = data_offset + data_padded.len() as u32;

        // Calculate total size including assets
        let has_assets = self.icon.is_some() || self.nacp.is_some() || self.romfs.is_some();
        let total_size = if has_assets {
            let icon_size = self.icon.as_ref().map_or(0, |v| v.len());
            let nacp_size = self.nacp.as_ref().map_or(0, |v| v.len());
            let romfs_size = self.romfs.as_ref().map_or(0, |v| v.len());
            nro_size as usize + 0x38 + icon_size + nacp_size + romfs_size
        } else {
            nro_size as usize
        };

        let mut buf = Vec::with_capacity(total_size);

        // Write NroStart (0x10 bytes)
        let mut start = NroStart::new_zeroed();
        start.unused = 0.into();
        start.mod_offset = self.mod0_offset.unwrap_or(0).into();
        buf.extend_from_slice(zerocopy::IntoBytes::as_bytes(&start));

        // Write NroHeader (0x70 bytes)
        let mut header = NroHeader::new_zeroed();
        header.magic = NRO_MAGIC.into();
        header.version = 0.into();
        header.size = nro_size.into();
        header.flags = 0.into();
        header.segments = [
            NroSegment {
                file_off: text_offset.into(),
                size: (text.len() as u32).into(),
            },
            NroSegment {
                file_off: rodata_offset.into(),
                size: (rodata.len() as u32).into(),
            },
            NroSegment {
                file_off: data_offset.into(),
                size: (data.len() as u32).into(),
            },
        ];
        header.bss_size = self.bss_size.into();
        header.build_id = self.build_id.unwrap_or([0u8; 0x20]);
        buf.extend_from_slice(zerocopy::IntoBytes::as_bytes(&header));

        // Write padded segments
        buf.extend_from_slice(&text_padded);
        buf.extend_from_slice(&rodata_padded);
        buf.extend_from_slice(&data_padded);

        // Write asset header and assets if present
        if has_assets {
            let mut asset_offset = 0x38u64; // Header size

            let (icon_off, icon_size) = if let Some(icon) = &self.icon {
                let off = asset_offset;
                asset_offset += icon.len() as u64;
                (off, icon.len() as u64)
            } else {
                (0, 0)
            };

            let (nacp_off, nacp_size) = if let Some(nacp) = &self.nacp {
                let off = asset_offset;
                asset_offset += nacp.len() as u64;
                (off, nacp.len() as u64)
            } else {
                (0, 0)
            };

            let (romfs_off, romfs_size) = if let Some(romfs) = &self.romfs {
                let off = asset_offset;
                (off, romfs.len() as u64)
            } else {
                (0, 0)
            };

            let asset_header = NroAssetHeader {
                magic: ASSET_MAGIC.into(),
                version: 0.into(),
                icon: crate::raw::nro::NroAssetSection {
                    offset: icon_off.into(),
                    size: icon_size.into(),
                },
                nacp: crate::raw::nro::NroAssetSection {
                    offset: nacp_off.into(),
                    size: nacp_size.into(),
                },
                romfs: crate::raw::nro::NroAssetSection {
                    offset: romfs_off.into(),
                    size: romfs_size.into(),
                },
            };
            buf.extend_from_slice(zerocopy::IntoBytes::as_bytes(&asset_header));

            // Write asset data
            if let Some(icon) = &self.icon {
                buf.extend_from_slice(icon);
            }
            if let Some(nacp) = &self.nacp {
                buf.extend_from_slice(nacp);
            }
            if let Some(romfs) = &self.romfs {
                buf.extend_from_slice(romfs);
            }
        }

        Ok(buf)
    }
}

impl Default for NroBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned by [`NroBuilder::build`].
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
