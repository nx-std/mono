//! ELF segment extraction for Nintendo Switch executables.

use std::vec::Vec;

use object::{
    Endianness, Object, ObjectSection, ObjectSegment,
    elf::{FileHeader64, NT_GNU_BUILD_ID, PT_LOAD, SHT_DYNAMIC, SHT_DYNSYM, SHT_NOTE, SHT_STRTAB},
    read::elf::{ElfFile64, FileHeader, ProgramHeader, SectionHeader},
};

use crate::write::{NroBuilder, NsoBuilder};

/// Information about an ELF section.
#[derive(Debug, Clone, Copy)]
pub struct SectionInfo {
    /// Virtual address of the section.
    pub addr: u64,
    /// Size of the section in bytes.
    pub size: u64,
}

/// Parsed ELF segments ready for NRO/NSO generation.
pub struct ElfSegments {
    text: Vec<u8>,
    rodata: Vec<u8>,
    data: Vec<u8>,
    bss_size: u64,
    build_id: Option<[u8; 0x20]>,
    mod0_offset: Option<u32>,
    dynamic: Option<SectionInfo>,
    dynstr: Option<SectionInfo>,
    dynsym: Option<SectionInfo>,
    eh_frame_hdr: Option<SectionInfo>,
}

impl ElfSegments {
    /// Parse an ELF file and extract segments for NRO/NSO generation.
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let elf = ElfFile64::<Endianness>::parse(data)?;

        // Verify architecture
        if elf.architecture() != object::Architecture::Aarch64 {
            return Err(ParseError::UnsupportedArch);
        }

        // Extract PT_LOAD segments
        let mut segments: Vec<_> = elf
            .segments()
            .filter_map(|seg| {
                if seg.p_type() == PT_LOAD {
                    Some((seg.address(), seg.data().ok()?))
                } else {
                    None
                }
            })
            .collect();

        // Sort by address
        segments.sort_by_key(|(addr, _)| *addr);

        // Extract text, rodata, data
        let text = segments.get(0).ok_or(ParseError::MissingText)?.1.to_vec();
        let rodata = segments.get(1).ok_or(ParseError::MissingRodata)?.1.to_vec();
        let data = segments.get(2).ok_or(ParseError::MissingData)?.1.to_vec();

        // Extract BSS size from optional 4th segment or from data segment memsz
        let bss_size = if let Some((_, seg_data)) = segments.get(3) {
            seg_data.len() as u64
        } else {
            // Check if data segment has memsz > filesz (embedded BSS)
            let data_seg = elf.segments().nth(2).ok_or(ParseError::MissingData)?;
            let memsz = data_seg.size();
            let filesz = data_seg.file_range().1;
            if memsz > filesz {
                let aligned_filesz = (filesz + 0xFFF) & !0xFFF;
                if memsz > aligned_filesz {
                    ((memsz - aligned_filesz) + 0xFFF) & !0xFFF
                } else {
                    0
                }
            } else {
                0
            }
        };

        // Extract build ID from SHT_NOTE
        let build_id = extract_build_id(&elf);

        // Extract MOD0 offset from text segment
        let mod0_offset = if text.len() >= 8 {
            let offset_bytes: [u8; 4] = text[4..8].try_into().unwrap();
            let offset = u32::from_le_bytes(offset_bytes);
            if offset > 0 && offset < text.len() as u32 {
                Some(offset)
            } else {
                None
            }
        } else {
            None
        };

        // Extract section information
        let mut dynamic = None;
        let mut dynstr = None;
        let mut dynsym = None;
        let mut eh_frame_hdr = None;

        for section in elf.sections() {
            if let Ok(name) = section.name() {
                let info = SectionInfo {
                    addr: section.address(),
                    size: section.size(),
                };

                match name {
                    ".dynamic" => dynamic = Some(info),
                    ".dynstr" => dynstr = Some(info),
                    ".dynsym" => dynsym = Some(info),
                    ".eh_frame_hdr" => eh_frame_hdr = Some(info),
                    _ => {}
                }
            }
        }

        Ok(ElfSegments {
            text,
            rodata,
            data,
            bss_size,
            build_id,
            mod0_offset,
            dynamic,
            dynstr,
            dynsym,
            eh_frame_hdr,
        })
    }

    /// Get the text (code) segment.
    pub fn text(&self) -> &[u8] {
        &self.text
    }

    /// Get the rodata (read-only data) segment.
    pub fn rodata(&self) -> &[u8] {
        &self.rodata
    }

    /// Get the data (read-write data) segment.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the BSS section size in bytes.
    pub fn bss_size(&self) -> u64 {
        self.bss_size
    }

    /// Get the 32-byte build ID, if present.
    pub fn build_id(&self) -> Option<&[u8; 0x20]> {
        self.build_id.as_ref()
    }

    /// Get the MOD0 offset relative to the start of the text segment.
    pub fn mod0_offset(&self) -> Option<u32> {
        self.mod0_offset
    }

    /// Convert into an NroBuilder with segments pre-populated.
    pub fn into_nro_builder(self) -> NroBuilder {
        let mut builder = NroBuilder::new()
            .text(self.text)
            .rodata(self.rodata)
            .data(self.data)
            .bss_size(self.bss_size as u32);

        if let Some(build_id) = self.build_id {
            builder = builder.build_id(build_id);
        }

        if let Some(offset) = self.mod0_offset {
            builder = builder.mod0_offset(offset);
        }

        builder
    }

    /// Convert into an NsoBuilder with segments pre-populated.
    pub fn into_nso_builder(self) -> NsoBuilder {
        let mut builder = NsoBuilder::new()
            .text(self.text)
            .rodata(self.rodata)
            .data(self.data)
            .bss_size(self.bss_size as u32);

        if let Some(build_id) = self.build_id {
            builder = builder.module_id(build_id);
        }

        builder
    }
}

/// Error returned by [`ElfSegments::parse`].
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Invalid ELF file.
    #[error("invalid ELF: {0}")]
    InvalidElf(#[from] object::read::Error),
    /// Unsupported architecture (expected AArch64).
    #[error("unsupported architecture: expected AArch64")]
    UnsupportedArch,
    /// Missing text segment.
    #[error("missing text segment")]
    MissingText,
    /// Missing rodata segment.
    #[error("missing rodata segment")]
    MissingRodata,
    /// Missing data segment.
    #[error("missing data segment")]
    MissingData,
}

/// Extract build ID from SHT_NOTE sections.
fn extract_build_id(elf: &ElfFile64<Endianness>) -> Option<[u8; 0x20]> {
    for section in elf.sections() {
        if section.kind() == object::SectionKind::Note {
            if let Ok(data) = section.data() {
                // Parse note header: namesz (4), descsz (4), type (4), name, desc
                if data.len() < 12 {
                    continue;
                }

                let _namesz = u32::from_le_bytes(data[0..4].try_into().ok()?);
                let descsz = u32::from_le_bytes(data[4..8].try_into().ok()?);
                let note_type = u32::from_le_bytes(data[8..12].try_into().ok()?);

                // NT_GNU_BUILD_ID = 3
                if note_type == NT_GNU_BUILD_ID && descsz >= 0x20 {
                    // Skip past nhdr (12 bytes) to get to descriptor
                    // Build ID starts at offset 0x10 in the note data (after 16-byte nhdr)
                    if data.len() >= 0x30 {
                        let mut build_id = [0u8; 0x20];
                        build_id.copy_from_slice(&data[0x10..0x30]);
                        return Some(build_id);
                    }
                }
            }
        }
    }
    None
}
