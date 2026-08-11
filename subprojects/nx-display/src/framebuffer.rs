//! A CPU-drawn surface a compositor can display.
//!
//! Bringing one up is four things in a fixed order: work out how big the
//! pixels really have to be, allocate that, hand the allocation to the driver
//! as a memory object, and register each buffer of it with the window. After
//! that a frame is [`Framebuffer::begin`], draw, [`Framebuffer::end`].
//!
//! ## The allocation is bigger than the picture
//!
//! Display hardware reads tiles, not rows, so a surface is rounded out to
//! whole tiles in both directions before anything is allocated: rows to the
//! 64-byte tile width, columns to the 128-pixel block height. A 1280x720
//! surface is stored as 1280x768. [`Geometry`] is where that arithmetic lives,
//! and it is separated from everything else here because it is the part that
//! can be checked without a console.
//!
//! ## Tiled is the storage order, not the drawing order
//!
//! Drawing into a tiled buffer directly means computing a swizzled address per
//! pixel. [`Framebuffer::make_linear`] offers the alternative: draw into a
//! plain row-major buffer and have [`Framebuffer::end`] convert it. That costs
//! a copy per frame and a second allocation, which is why it is opt-in.
//!
//! ## No fence is waited on
//!
//! The producer hands back a fence when it releases a buffer, and this does
//! not wait on it. That is sound only because the surface is written by the
//! CPU and flushed from the cache before it is queued: there is no GPU work in
//! flight against these pages to be ordered against. A future caller that
//! renders with the GPU needs the fence, and needs it before it needs anything
//! else here.

use alloc::alloc::{
    Layout as AllocLayout,
    alloc_zeroed,
    dealloc,
};
use core::ptr::NonNull;

use nx_nv::{
    BorrowedMapDevice,
    ColorFormat,
    GraphicBuffer,
    GraphicBufferParams,
    MapAlign,
    MapBuffer,
    MapKind,
    MemoryMap,
    USAGE_HW_COMPOSER,
    USAGE_HW_RENDER,
    USAGE_HW_TEXTURE,
};
use nx_service_vi::igbp::BqGraphicBufferInput;

use crate::native_window::{
    NativeWindow,
    NativeWindowError,
};

/// Tiles are 64 bytes wide.
const GOB_WIDTH_BYTES: u32 = 64;

/// Tiles are 8 pixels tall.
const GOB_HEIGHT_PX: u32 = 8;

/// Log2 of how many tile rows make up one block; sixteen is what the display
/// hardware reads most efficiently.
const BLOCK_HEIGHT_LOG2: u32 = 4;

/// One block is this many pixels tall.
const BLOCK_HEIGHT_PX: u32 = GOB_HEIGHT_PX << BLOCK_HEIGHT_LOG2;

/// One tile is this many bytes.
const GOB_SIZE_BYTES: usize = 512;

/// The page size the allocation is rounded to.
const PAGE_SIZE: u32 = 0x1000;

/// The alignment the memory object's pages are placed at.
const MAP_ALIGN: u32 = 0x2_0000;

/// How many buffers a surface may be split into.
const MAX_BUFFERS: u32 = 3;

/// The pixel encoding a surface is drawn and displayed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelFormat {
    producer_code: u32,
    color: ColorFormat,
}

impl PixelFormat {
    /// 8 bits each of red, green, blue and alpha.
    pub const RGBA_8888: Self = Self {
        producer_code: 1,
        color: ColorFormat::A8B8G8R8,
    };

    /// As [`PixelFormat::RGBA_8888`] with the alpha byte ignored.
    pub const RGBX_8888: Self = Self {
        producer_code: 2,
        color: ColorFormat::X8B8G8R8,
    };

    /// 5 bits red, 6 green, 5 blue.
    pub const RGB_565: Self = Self {
        producer_code: 4,
        color: ColorFormat::R5G6B5,
    };

    /// 8 bits each of blue, green, red and alpha.
    pub const BGRA_8888: Self = Self {
        producer_code: 5,
        color: ColorFormat::A8R8G8B8,
    };

    /// 4 bits each of red, green, blue and alpha.
    pub const RGBA_4444: Self = Self {
        producer_code: 7,
        color: ColorFormat::A4B4G4R4,
    };

    /// Returns how many bytes one pixel occupies.
    #[inline]
    pub const fn bytes_per_pixel(self) -> u32 {
        self.color.bytes_per_pixel()
    }
}

/// How much memory a surface actually occupies, and how it is divided.
///
/// Every field here follows from the requested size and the tile shape, so
/// this is worked out once, up front, and read from afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// The row stride in pixels, rounded out to whole tiles.
    pub width_aligned: u32,
    /// The row stride in bytes, rounded out to whole tiles.
    pub stride: u32,
    /// The height in pixels, rounded out to whole blocks.
    pub height_aligned: u32,
    /// One buffer's size in bytes.
    pub buffer_size: u32,
    /// Every buffer together, rounded out to whole pages.
    pub total_size: u32,
}

impl Geometry {
    /// Works out the storage a surface of `width` by `height` needs.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when the requested size is empty, the buffer
    /// count is outside one to three, or the rounded-out surface does not fit
    /// the 32-bit size fields the descriptor carries.
    pub fn plan(
        width: u32,
        height: u32,
        format: PixelFormat,
        buffers: u32,
    ) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            return Err(GeometryError::Empty { width, height });
        }
        if buffers == 0 || buffers > MAX_BUFFERS {
            return Err(GeometryError::BufferCount { buffers });
        }

        let bytes_per_pixel = format.bytes_per_pixel();
        let row_bytes = width
            .checked_mul(bytes_per_pixel)
            .ok_or(GeometryError::TooLarge)?;
        let stride = align_up(row_bytes, GOB_WIDTH_BYTES).ok_or(GeometryError::TooLarge)?;
        let height_aligned = align_up(height, BLOCK_HEIGHT_PX).ok_or(GeometryError::TooLarge)?;

        let buffer_size = stride
            .checked_mul(height_aligned)
            .ok_or(GeometryError::TooLarge)?;
        let total_size = buffer_size
            .checked_mul(buffers)
            .and_then(|bytes| align_up(bytes, PAGE_SIZE))
            .ok_or(GeometryError::TooLarge)?;

        Ok(Self {
            width_aligned: stride / bytes_per_pixel,
            stride,
            height_aligned,
            buffer_size,
            total_size,
        })
    }
}

/// Rounds `value` up to the next multiple of `to`, which must be a power of two.
const fn align_up(value: u32, to: u32) -> Option<u32> {
    match value.checked_add(to - 1) {
        Some(raised) => Some(raised & !(to - 1)),
        None => None,
    }
}

/// Errors returned by [`Geometry::plan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    /// The requested surface has no pixels
    #[error("A {width}x{height} surface has no pixels")]
    Empty {
        /// The requested width.
        width: u32,
        /// The requested height.
        height: u32,
    },

    /// The buffer count is outside what a surface may be split into
    #[error("A surface holds one to three buffers, not {buffers}")]
    BufferCount {
        /// The rejected count.
        buffers: u32,
    },

    /// The rounded-out surface does not fit the descriptor's size fields
    #[error("The surface is too large to describe")]
    TooLarge,
}

/// A surface the CPU draws into and a compositor displays.
pub struct Framebuffer<'a> {
    window: &'a NativeWindow,
    map: MemoryMap<'a>,
    pixels: Allocation,
    linear: Option<Allocation>,
    geometry: Geometry,
    buffers: u32,
    /// The slot `begin` took, until `end` hands it back.
    taken: Option<i32>,
}

impl<'a> Framebuffer<'a> {
    /// Brings up a surface of `width` by `height` on `window`.
    ///
    /// `buffers` is how many buffers to cycle through; two lets the CPU draw
    /// one frame while the compositor displays the previous one.
    ///
    /// # Errors
    ///
    /// Returns [`FramebufferError`] when the size is unworkable, the
    /// allocation fails, the driver refuses the memory object, or the window
    /// refuses a buffer.
    pub fn create(
        window: &'a NativeWindow,
        device: BorrowedMapDevice<'a>,
        width: u32,
        height: u32,
        format: PixelFormat,
        buffers: u32,
    ) -> Result<Self, FramebufferError> {
        let geometry =
            Geometry::plan(width, height, format, buffers).map_err(FramebufferError::Geometry)?;

        window
            .set_dimensions(width, height)
            .map_err(FramebufferError::SetDimensions)?;

        let pixels = Allocation::create(geometry.total_size as usize, PAGE_SIZE as usize)
            .ok_or(FramebufferError::OutOfMemory)?;

        let buffer = MapBuffer::create(pixels.ptr, pixels.len).map_err(FramebufferError::Buffer)?;
        let align = MapAlign::try_from(MAP_ALIGN).map_err(FramebufferError::Align)?;
        // The object is created as untiled and cacheable: the tiling is a
        // property of the plane the descriptor declares, not of the pages, and
        // the CPU writes these pages every frame.
        let map = device
            .create_map(buffer, align, MapKind::PITCH, true)
            .map_err(FramebufferError::CreateMap)?;

        let framebuffer = Self {
            window,
            map,
            pixels,
            linear: None,
            geometry,
            buffers,
            taken: None,
        };
        framebuffer.configure_slots(format)?;
        Ok(framebuffer)
    }

    /// Registers every buffer of the surface with the window.
    fn configure_slots(&self, format: PixelFormat) -> Result<(), FramebufferError> {
        let usage = USAGE_HW_COMPOSER | USAGE_HW_RENDER | USAGE_HW_TEXTURE;

        for slot in 0..self.buffers {
            let descriptor = GraphicBuffer::new(&GraphicBufferParams {
                map_id: self.map.id(),
                width: self.window.dimensions().0,
                height: self.window.dimensions().1,
                stride: self.geometry.width_aligned,
                pitch: self.geometry.stride,
                offset: slot * self.geometry.buffer_size,
                plane_size: self.geometry.buffer_size,
                color_format: format.color,
                format: format.producer_code,
                kind: MapKind::GENERIC_16BX2,
                block_height_log2: BLOCK_HEIGHT_LOG2,
                usage,
            });

            let input = BqGraphicBufferInput {
                width: self.window.dimensions().0,
                height: self.window.dimensions().1,
                stride: self.geometry.width_aligned,
                format: format.producer_code,
                usage,
                native_handle_ints: descriptor.as_ints(),
            };

            self.window
                .configure_buffer(slot as i32, &input)
                .map_err(FramebufferError::ConfigureBuffer)?;
        }
        Ok(())
    }

    /// Adds a row-major buffer to draw into, converted on [`Framebuffer::end`].
    ///
    /// # Errors
    ///
    /// Returns [`MakeLinearError`] when one is already present, or when the
    /// second allocation fails.
    pub fn make_linear(&mut self) -> Result<(), MakeLinearError> {
        if self.linear.is_some() {
            return Err(MakeLinearError::AlreadyLinear);
        }

        let height = align_up(self.window.dimensions().1, GOB_HEIGHT_PX)
            .ok_or(MakeLinearError::OutOfMemory)?;
        let size = self
            .geometry
            .stride
            .checked_mul(height)
            .ok_or(MakeLinearError::OutOfMemory)? as usize;

        self.linear =
            Some(Allocation::create(size, PAGE_SIZE as usize).ok_or(MakeLinearError::OutOfMemory)?);
        Ok(())
    }

    /// Returns the surface's storage layout.
    #[inline]
    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    /// Takes the next buffer to draw into, waiting for one to come free.
    ///
    /// The returned slice is row-major when [`Framebuffer::make_linear`] was
    /// called and tiled otherwise, with rows [`Geometry::stride`] bytes apart.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError`] when the producer refuses to release a buffer.
    pub fn begin(&mut self) -> Result<&mut [u8], FrameError> {
        let dequeued = self
            .window
            .dequeue_buffer()
            .map_err(FrameError::DequeueBuffer)?;

        self.taken = Some(dequeued.slot);

        if let Some(linear) = self.linear.as_mut() {
            return Ok(linear.as_mut_slice());
        }

        let offset = dequeued.slot as usize * self.geometry.buffer_size as usize;
        let len = self.geometry.buffer_size as usize;
        Ok(&mut self.pixels.as_mut_slice()[offset..offset + len])
    }

    /// Hands the buffer back for display.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError`] when no buffer is currently taken, or the
    /// producer refuses it.
    pub fn end(&mut self) -> Result<(), FrameError> {
        let slot = self.taken.take().ok_or(FrameError::NoBufferTaken)?;

        let offset = slot as usize * self.geometry.buffer_size as usize;
        let len = self.geometry.buffer_size as usize;

        if let Some(linear) = self.linear.as_ref() {
            let height = self.window.dimensions().1;
            let source = linear.as_slice();
            let target = &mut self.pixels.as_mut_slice()[offset..offset + len];
            convert_to_block_linear(target, source, self.geometry.stride, height);
        }

        // The compositor reads these pages without going through the CPU's
        // caches, so what it sees is whatever main memory holds — the frame
        // has to be written back before it is handed over.
        // SAFETY: the range is inside the allocation this framebuffer owns.
        unsafe {
            let base = self.pixels.ptr.as_ptr().add(offset);
            nx_cpu::cache::flush_data_range(base, len);
        }

        self.window
            .queue_buffer(slot, None)
            .map_err(FrameError::QueueBuffer)?;
        Ok(())
    }
}

impl Drop for Framebuffer<'_> {
    fn drop(&mut self) {
        // The window holds buffers that name these pages, so it has to let
        // them go before the pages do. A refusal cannot be reported from a
        // destructor and cannot be retried.
        let _ = self.window.release_buffers();
    }
}

/// Errors returned by [`Framebuffer::create`].
#[derive(Debug, thiserror::Error)]
pub enum FramebufferError {
    /// The requested surface cannot be laid out
    #[error("Failed to lay out the surface")]
    Geometry(#[source] GeometryError),

    /// The window refused the surface's dimensions
    #[error("Failed to set the window dimensions")]
    SetDimensions(#[source] NativeWindowError),

    /// The surface's pages could not be allocated
    #[error("Failed to allocate the surface")]
    OutOfMemory,

    /// The allocation cannot back a memory object
    #[error("The allocation cannot back a memory object")]
    Buffer(#[source] nx_nv::MapBufferError),

    /// The memory object's alignment was refused
    #[error("The memory object alignment was refused")]
    Align(#[source] nx_nv::MapAlignError),

    /// The driver refused the memory object
    #[error("Failed to create the memory object")]
    CreateMap(#[source] nx_nv::CreateMapError),

    /// The window refused one of the surface's buffers
    #[error("Failed to register a buffer with the window")]
    ConfigureBuffer(#[source] NativeWindowError),
}

/// Errors returned by [`Framebuffer::make_linear`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MakeLinearError {
    /// A row-major buffer is already present
    #[error("The framebuffer already draws row-major")]
    AlreadyLinear,

    /// The row-major buffer could not be allocated
    #[error("Failed to allocate the row-major buffer")]
    OutOfMemory,
}

/// Errors returned by [`Framebuffer::begin`] and [`Framebuffer::end`].
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// The producer would not release a buffer
    #[error("Failed to take a buffer to draw into")]
    DequeueBuffer(#[source] NativeWindowError),

    /// No buffer is currently taken
    #[error("No buffer is currently taken")]
    NoBufferTaken,

    /// The producer refused the finished frame
    #[error("Failed to hand the frame back for display")]
    QueueBuffer(#[source] NativeWindowError),
}

/// A page-aligned allocation the framebuffer owns.
struct Allocation {
    ptr: NonNull<u8>,
    len: usize,
    layout: AllocLayout,
}

impl Allocation {
    /// Allocates `len` zeroed bytes at `align`.
    fn create(len: usize, align: usize) -> Option<Self> {
        let layout = AllocLayout::from_size_align(len, align).ok()?;
        // SAFETY: the layout has a non-zero size, since a zero-sized surface
        // is rejected before this is reached.
        let ptr = NonNull::new(unsafe { alloc_zeroed(layout) })?;
        Some(Self { ptr, len, layout })
    }

    fn as_slice(&self) -> &[u8] {
        // SAFETY: the allocation is live and `len` bytes long.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: the allocation is live, `len` bytes long, and borrowed
        // exclusively for the length of the returned slice.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `alloc_zeroed` with this same layout
        // and has not been freed.
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) };
    }
}

/// Rewrites one tile from row-major order into the order the display reads.
///
/// A tile's byte offset is nine bits, `yyyxxxxxx`. The hardware swizzles the
/// top five: `43210` becomes `14302`. The bottom four are untouched, so each
/// step moves sixteen bytes as a unit.
fn convert_tile(target: &mut [u8], source: &[u8], stride: u32) {
    for step in 0..32_usize {
        let y = ((step >> 1) & 0x06) | (step & 0x01);
        let x = ((step << 3) & 0x10) | ((step << 1) & 0x20);

        let from = y * stride as usize + x;
        let to = step * 16;
        target[to..to + 16].copy_from_slice(&source[from..from + 16]);
    }
}

/// Rewrites a row-major surface into the order the display reads.
fn convert_to_block_linear(target: &mut [u8], source: &[u8], stride: u32, height: u32) {
    let block_height_gobs = 1_usize << BLOCK_HEIGHT_LOG2;
    let width_blocks = (stride >> 6) as usize;
    let height_blocks = ((height + BLOCK_HEIGHT_PX - 1) >> (3 + BLOCK_HEIGHT_LOG2)) as usize;

    let mut at = 0_usize;
    for block_y in 0..height_blocks {
        for block_x in 0..width_blocks {
            for gob_y in 0..block_height_gobs {
                let x = block_x * GOB_WIDTH_BYTES as usize;
                let y = block_y * BLOCK_HEIGHT_PX as usize + gob_y * GOB_HEIGHT_PX as usize;

                if (y as u32) < height && at + GOB_SIZE_BYTES <= target.len() {
                    let from = y * stride as usize + x;
                    // A row-major buffer shorter than the tiled one it fills
                    // is not an error: the tail tiles cover rows past the
                    // visible height and are left as they were.
                    if from + 7 * stride as usize + GOB_WIDTH_BYTES as usize <= source.len() {
                        convert_tile(
                            &mut target[at..at + GOB_SIZE_BYTES],
                            &source[from..],
                            stride,
                        );
                    }
                }
                at += GOB_SIZE_BYTES;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_720p_surface_is_stored_768_rows_tall() {
        //* Given
        let format = PixelFormat::RGBA_8888;

        //* When
        let geometry = Geometry::plan(1280, 720, format, 2).expect("720p should lay out");

        //* Then
        assert_eq!(geometry.height_aligned, 768, "720 rounds up to six blocks");
        assert_eq!(
            geometry.stride, 5120,
            "1280 four-byte pixels already fill whole tiles"
        );
        assert_eq!(
            geometry.width_aligned, 1280,
            "no horizontal padding is needed"
        );
    }

    #[test]
    fn a_width_that_does_not_fill_a_tile_is_padded_out() {
        //* Given
        let format = PixelFormat::RGBA_8888;

        //* When
        let geometry = Geometry::plan(100, 100, format, 1).expect("a small surface should lay out");

        //* Then
        assert_eq!(geometry.stride, 448, "400 bytes rounds up to seven tiles");
        assert_eq!(
            geometry.width_aligned, 112,
            "the padded stride is 112 pixels"
        );
        assert_eq!(
            geometry.height_aligned, 128,
            "100 rows rounds up to one block"
        );
    }

    #[test]
    fn the_total_is_every_buffer_rounded_to_whole_pages() {
        //* Given
        let format = PixelFormat::RGB_565;

        //* When
        let geometry = Geometry::plan(320, 240, format, 3).expect("a small surface should lay out");

        //* Then
        assert_eq!(
            geometry.total_size % PAGE_SIZE,
            0,
            "the driver pins whole pages"
        );
        assert!(
            geometry.total_size >= geometry.buffer_size * 3,
            "every buffer must fit"
        );
    }

    #[test]
    fn an_empty_surface_is_rejected() {
        //* Given
        let format = PixelFormat::RGBA_8888;

        //* When
        let result = Geometry::plan(0, 720, format, 2);

        //* Then
        assert_eq!(
            result,
            Err(GeometryError::Empty {
                width: 0,
                height: 720
            }),
            "a surface with no pixels cannot be laid out"
        );
    }

    #[test]
    fn a_fourth_buffer_is_rejected() {
        //* Given
        let format = PixelFormat::RGBA_8888;

        //* When
        let result = Geometry::plan(1280, 720, format, 4);

        //* Then
        assert_eq!(
            result,
            Err(GeometryError::BufferCount { buffers: 4 }),
            "the producer tracks at most three buffers"
        );
    }

    #[test]
    fn tile_conversion_moves_every_byte_of_the_tile() {
        //* Given
        let stride = 64_u32;
        let source: [u8; 512] = core::array::from_fn(|at| at as u8);
        let mut target = [0_u8; 512];

        //* When
        convert_tile(&mut target, &source, stride);

        //* Then
        let mut seen = [false; 512];
        for byte in target {
            seen[byte as usize] = true;
        }
        assert!(
            seen.iter().all(|hit| *hit),
            "a swizzle is a permutation: every source byte must appear once"
        );
    }

    #[test]
    fn tile_conversion_places_the_first_sector_first() {
        //* Given
        let stride = 64_u32;
        let source: [u8; 512] = core::array::from_fn(|at| at as u8);
        let mut target = [0_u8; 512];

        //* When
        convert_tile(&mut target, &source, stride);

        //* Then
        assert_eq!(
            &target[..16],
            &source[..16],
            "the first sector is at the origin in both orders"
        );
    }
}
