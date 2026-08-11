//! The buffer descriptor a compositor is handed alongside a memory object.
//!
//! A memory object gives the compositor pages; this says what is in them —
//! the pixel format, the tiling, the stride, and where each buffer of a
//! multi-buffered surface starts. It travels as the "ints" payload of the
//! native handle inside a producer transaction, which is why the whole thing
//! is a fixed-layout struct rather than a serializer: the receiver reads it by
//! offset.
//!
//! ## The colour format is 64 bits wide
//!
//! Its codes carry the bytes-per-pixel in bits 3..8 and run past 32 bits, so
//! the field is eight bytes and everything after it in [`Surface`] sits four
//! bytes further along than a reader counting 32-bit fields would guess. That
//! single fact is the most likely way to get this struct subtly wrong, so the
//! layout is pinned by assertion below rather than trusted.
//!
//! ## The header is part of the layout, not a prefix to skip
//!
//! The descriptor is preceded by a three-word native-handle header, and the
//! plane array is eight-byte aligned relative to *that* start. Modelling the
//! payload alone would align the planes four bytes earlier and describe a
//! buffer nobody can read, so the header is carried here and dropped at the
//! point of serialization instead.

use crate::map::{
    MapId,
    MapKind,
};

/// How a surface's pixels are arranged in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Layout(u32);

impl Layout {
    /// Rows laid out one after another.
    pub const PITCH: Self = Self(1);

    /// Rows grouped into tiles, which is what the display hardware reads.
    pub const BLOCK_LINEAR: Self = Self(3);
}

/// Whether a surface is scanned out in one pass or two interleaved ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ScanFormat(u32);

impl ScanFormat {
    /// One pass, top to bottom.
    pub const PROGRESSIVE: Self = Self(0);
}

/// The pixel encoding of a surface.
///
/// The code carries the bytes-per-pixel in bits 3..8, which is why
/// [`ColorFormat::bytes_per_pixel`] can answer without a lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorFormat(u64);

impl ColorFormat {
    /// 8 bits each of alpha, blue, green and red.
    pub const A8B8G8R8: Self = Self(0x0001_0053_2120);

    /// As [`ColorFormat::A8B8G8R8`] with the alpha byte ignored.
    pub const X8B8G8R8: Self = Self(0x0001_0053_2121);

    /// 5 bits red, 6 green, 5 blue.
    pub const R5G6B5: Self = Self(0x0001_0053_2010);

    /// 8 bits each of alpha, red, green and blue.
    pub const A8R8G8B8: Self = Self(0x0001_0053_2125);

    /// 4 bits each of alpha, blue, green and red.
    pub const A4B4G4R4: Self = Self(0x0001_0053_2012);

    /// Returns how many bytes one pixel occupies.
    #[inline]
    pub const fn bytes_per_pixel(self) -> u32 {
        ((self.0 >> 3) & 0x1F) as u32
    }

    /// Returns the raw encoding code.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// One plane of a buffer: where it starts and how its pixels are arranged.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Surface {
    /// The plane's width in pixels.
    pub width: u32,
    /// The plane's height in pixels.
    pub height: u32,
    /// The pixel encoding, as [`ColorFormat`].
    pub color_format: u64,
    /// The tiling, as [`Layout`].
    pub layout: u32,
    /// The distance between rows in bytes.
    pub pitch: u32,
    /// Overwritten by the receiver; the sender leaves it zero.
    pub unused: u32,
    /// Where this plane starts inside the memory object.
    pub offset: u32,
    /// The memory layout the display hardware reads the plane with.
    pub kind: u32,
    /// Log2 of how many tile rows make up one block.
    pub block_height_log2: u32,
    /// The scan-out order, as [`ScanFormat`].
    pub scan: u32,
    /// Where the second field starts, for interleaved scan-out.
    pub second_field_offset: u32,
    /// Reserved; the sender leaves it zero.
    pub flags: u64,
    /// The plane's size in bytes.
    pub size: u64,
    /// Compression bookkeeping the sender leaves zero.
    pub unk: [u32; 6],
}

const _: () = {
    assert!(size_of::<Surface>() == 88);
    assert!(align_of::<Surface>() == 8);
    // The 64-bit colour format is what puts everything after it here.
    assert!(core::mem::offset_of!(Surface, color_format) == 8);
    assert!(core::mem::offset_of!(Surface, layout) == 16);
    assert!(core::mem::offset_of!(Surface, offset) == 28);
    assert!(core::mem::offset_of!(Surface, flags) == 48);
    assert!(core::mem::offset_of!(Surface, size) == 56);
};

/// The three-word header the descriptor travels behind.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NativeHandleHeader {
    /// The header's own size in bytes.
    pub version: i32,
    /// How many file descriptors follow; always zero here.
    pub num_fds: i32,
    /// How many 32-bit words of payload follow the header.
    pub num_ints: i32,
}

/// The descriptor handed to a compositor alongside a memory object.
///
/// Build one with [`GraphicBuffer::new`] and hand [`GraphicBuffer::as_ints`]
/// to the producer transaction that registers the buffer.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct GraphicBuffer {
    header: NativeHandleHeader,
    unk0: i32,
    nvmap_id: u32,
    unk2: u32,
    magic: u32,
    pid: u32,
    buffer_type: u32,
    usage: u32,
    format: u32,
    ext_format: u32,
    stride: u32,
    total_size: u32,
    num_planes: u32,
    unk12: u32,
    planes: [Surface; 3],
    unused: u64,
}

const _: () = {
    assert!(size_of::<GraphicBuffer>() == 336);
    assert!(align_of::<GraphicBuffer>() == 8);
    assert!(core::mem::offset_of!(GraphicBuffer, unk0) == 12);
    assert!(core::mem::offset_of!(GraphicBuffer, nvmap_id) == 16);
    // The plane array is aligned against the header's start, not the
    // payload's, which is why the header is modelled rather than skipped.
    assert!(core::mem::offset_of!(GraphicBuffer, planes) == 64);
    assert!(core::mem::offset_of!(GraphicBuffer, unused) == 328);
    assert!(HEADER_WORDS * 4 == size_of::<NativeHandleHeader>());
};

/// How many words the header occupies ahead of the payload.
const HEADER_WORDS: usize = 3;

/// How many payload words follow the header.
const PAYLOAD_WORDS: usize = (size_of::<GraphicBuffer>() - size_of::<NativeHandleHeader>()) / 4;

/// The value marking a descriptor as one of ours.
const MAGIC: u32 = 0xDAFF_CAFF;

/// The process id the descriptor carries, which the receiver does not check.
const PID: u32 = 42;

/// The buffer may be composited by the display hardware.
pub const USAGE_HW_COMPOSER: u32 = 0x800;

/// The buffer may be rendered into by the GPU.
pub const USAGE_HW_RENDER: u32 = 0x200;

/// The buffer may be sampled as a texture by the GPU.
pub const USAGE_HW_TEXTURE: u32 = 0x100;

impl GraphicBuffer {
    /// Describes a single-plane buffer inside `map`.
    ///
    /// `offset` is where this buffer starts inside the memory object, which is
    /// how several buffers of one surface share a single allocation.
    pub fn new(params: &GraphicBufferParams) -> Self {
        let mut plane = Surface {
            width: params.width,
            height: params.height,
            color_format: params.color_format.to_raw(),
            layout: Layout::BLOCK_LINEAR.0,
            pitch: params.pitch,
            offset: params.offset,
            kind: u32::from(params.kind.to_raw()),
            block_height_log2: params.block_height_log2,
            scan: ScanFormat::PROGRESSIVE.0,
            size: u64::from(params.plane_size),
            ..Surface::default()
        };
        plane.unused = 0;

        Self {
            header: NativeHandleHeader {
                version: size_of::<NativeHandleHeader>() as i32,
                num_fds: 0,
                num_ints: PAYLOAD_WORDS as i32,
            },
            unk0: -1,
            nvmap_id: params.map_id.to_raw(),
            unk2: 0,
            magic: MAGIC,
            pid: PID,
            buffer_type: 0,
            usage: params.usage,
            format: params.format,
            ext_format: params.format,
            stride: params.stride,
            total_size: params.plane_size,
            num_planes: 1,
            unk12: 0,
            planes: [plane, Surface::default(), Surface::default()],
            unused: 0,
        }
    }

    /// Returns the payload words the producer transaction carries.
    ///
    /// The header is dropped here rather than left out of the struct, because
    /// it is what the plane array's alignment is measured against.
    pub fn as_ints(&self) -> &[u32] {
        let words: &[u32; size_of::<Self>() / 4] = zerocopy::transmute_ref!(self);
        &words[HEADER_WORDS..]
    }
}

/// What [`GraphicBuffer::new`] needs to describe a buffer.
///
/// A struct rather than a parameter list because eleven values of four types
/// in a row is a call nobody can read, and two of them are byte counts that
/// would silently swap.
#[derive(Debug, Clone, Copy)]
pub struct GraphicBufferParams {
    /// The memory object holding the pixels.
    pub map_id: MapId,
    /// The surface's width in pixels.
    pub width: u32,
    /// The surface's height in pixels.
    pub height: u32,
    /// The distance between rows in pixels.
    pub stride: u32,
    /// The distance between rows in bytes.
    pub pitch: u32,
    /// Where this buffer starts inside the memory object.
    pub offset: u32,
    /// One buffer's size in bytes.
    pub plane_size: u32,
    /// The pixel encoding.
    pub color_format: ColorFormat,
    /// The producer-facing pixel format code.
    pub format: u32,
    /// The memory layout the display hardware reads the plane with.
    pub kind: MapKind,
    /// Log2 of how many tile rows make up one block.
    pub block_height_log2: u32,
    /// What the buffer may be used for.
    pub usage: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> GraphicBufferParams {
        GraphicBufferParams {
            map_id: MapId::from_raw(7),
            width: 1280,
            height: 720,
            stride: 1280,
            pitch: 5120,
            offset: 0,
            plane_size: 0x38_4000,
            color_format: ColorFormat::A8B8G8R8,
            format: 1,
            kind: MapKind::GENERIC_16BX2,
            block_height_log2: 4,
            usage: USAGE_HW_COMPOSER,
        }
    }

    #[test]
    fn payload_word_count_matches_the_declared_header() {
        //* Given
        let buffer = GraphicBuffer::new(&params());

        //* When
        let ints = buffer.as_ints();

        //* Then
        assert_eq!(
            ints.len(),
            buffer.header.num_ints as usize,
            "the header must count the words that actually follow it"
        );
        assert_eq!(
            ints.len(),
            81,
            "the descriptor is 81 words behind its header"
        );
    }

    #[test]
    fn payload_starts_at_the_field_after_the_header() {
        //* Given
        let buffer = GraphicBuffer::new(&params());

        //* When
        let ints = buffer.as_ints();

        //* Then
        assert_eq!(ints[0], u32::MAX, "the first payload word is the -1 marker");
        assert_eq!(ints[1], 7, "the memory object id follows it");
        assert_eq!(ints[3], MAGIC, "the magic sits four words in");
    }

    #[test]
    fn a8b8g8r8_is_four_bytes_per_pixel() {
        //* Given
        let format = ColorFormat::A8B8G8R8;

        //* When
        let bpp = format.bytes_per_pixel();

        //* Then
        assert_eq!(bpp, 4, "eight bits each of four channels is four bytes");
    }

    #[test]
    fn r5g6b5_is_two_bytes_per_pixel() {
        //* Given
        let format = ColorFormat::R5G6B5;

        //* When
        let bpp = format.bytes_per_pixel();

        //* Then
        assert_eq!(bpp, 2, "sixteen bits of colour is two bytes");
    }

    #[test]
    fn the_plane_carries_the_geometry_it_was_given() {
        //* Given
        let source = params();

        //* When
        let buffer = GraphicBuffer::new(&source);

        //* Then
        let plane = buffer.planes[0];
        assert_eq!(plane.pitch, source.pitch, "the row stride is in bytes");
        assert_eq!(
            plane.layout,
            Layout::BLOCK_LINEAR.0,
            "display reads tiled pixels"
        );
        assert_eq!(
            plane.size,
            u64::from(source.plane_size),
            "the plane spans one buffer"
        );
        assert_eq!(buffer.num_planes, 1, "these surfaces are single-plane");
    }
}
