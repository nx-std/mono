//! Android Parcel implementation for Binder IPC.
//!
//! Parcels are used for serializing data in Binder transactions.
//! This implementation follows the Android Parcel format used by
//! IGraphicBufferProducer.

/// Maximum parcel payload size.
pub const PARCEL_MAX_PAYLOAD: usize = 0x400;

/// Parcel header structure.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ParcelHeader {
    /// Size of the payload data.
    pub payload_size: u32,
    /// Offset to payload data from start of parcel.
    pub payload_off: u32,
    /// Size of the objects data.
    pub objects_size: u32,
    /// Offset to objects data from start of parcel.
    pub objects_off: u32,
}

impl ParcelHeader {
    /// Size of the parcel header.
    pub const SIZE: usize = 16;
}

/// Parcel for Binder IPC serialization.
///
/// Used to serialize data for IGraphicBufferProducer transactions.
pub struct Parcel {
    /// Payload data buffer.
    payload: [u8; PARCEL_MAX_PAYLOAD],
    /// Current payload size (write position).
    payload_size: usize,
    /// Current read position.
    pos: usize,
}

impl Parcel {
    /// Creates a new empty Parcel.
    pub const fn new() -> Self {
        Self {
            payload: [0; PARCEL_MAX_PAYLOAD],
            payload_size: 0,
            pos: 0,
        }
    }

    /// Returns the current payload size.
    #[inline]
    pub fn payload_size(&self) -> usize {
        self.payload_size
    }

    /// Returns a reference to the payload data.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.payload_size]
    }

    /// Returns a mutable reference to the raw payload buffer.
    ///
    /// # Safety
    ///
    /// Caller must ensure data written is properly aligned and sized.
    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8; PARCEL_MAX_PAYLOAD] {
        &mut self.payload
    }

    /// Sets the payload size after external writes.
    ///
    /// # Safety
    ///
    /// Caller must ensure size does not exceed `PARCEL_MAX_PAYLOAD`.
    #[inline]
    pub fn set_payload_size(&mut self, size: usize) {
        debug_assert!(size <= PARCEL_MAX_PAYLOAD);
        self.payload_size = size;
    }

    /// Resets the read position to the beginning.
    #[inline]
    pub fn reset_read_pos(&mut self) {
        self.pos = 0;
    }

    /// Writes raw data to the parcel, aligned to 4 bytes.
    ///
    /// Returns a pointer to the written data, or `None` if there's not enough space.
    pub fn write_data(&mut self, data: &[u8]) -> Option<*mut u8> {
        let data_size = data.len();
        if data_size > i32::MAX as usize {
            return None;
        }

        // Align to 4 bytes
        let aligned_size = (data_size + 3) & !3;

        if self.payload_size + aligned_size > PARCEL_MAX_PAYLOAD {
            return None;
        }

        let ptr = self.payload[self.payload_size..].as_mut_ptr();
        if !data.is_empty() {
            // SAFETY: We checked bounds above.
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data_size);
            }
        }

        self.payload_size += aligned_size;
        Some(ptr)
    }

    /// Writes raw data and returns a mutable slice to fill.
    ///
    /// Returns `None` if there's not enough space.
    pub fn write_data_uninit(&mut self, size: usize) -> Option<&mut [u8]> {
        if size > i32::MAX as usize {
            return None;
        }

        let aligned_size = (size + 3) & !3;

        if self.payload_size + aligned_size > PARCEL_MAX_PAYLOAD {
            return None;
        }

        let start = self.payload_size;
        self.payload_size += aligned_size;
        Some(&mut self.payload[start..start + size])
    }

    /// Reads raw data from the parcel, aligned to 4 bytes.
    ///
    /// Returns a pointer to the data, or `None` if there's not enough data.
    pub fn read_data(&mut self, size: usize) -> Option<*const u8> {
        if size > i32::MAX as usize {
            return None;
        }

        let aligned_size = (size + 3) & !3;

        if self.pos + aligned_size > self.payload_size {
            return None;
        }

        let ptr = self.payload[self.pos..].as_ptr();
        self.pos += aligned_size;
        Some(ptr)
    }

    /// Writes a 32-bit signed integer.
    pub fn write_i32(&mut self, val: i32) {
        self.write_data(&val.to_ne_bytes());
    }

    /// Writes a 32-bit unsigned integer.
    pub fn write_u32(&mut self, val: u32) {
        self.write_data(&val.to_ne_bytes());
    }

    /// Writes a 64-bit signed integer.
    pub fn write_i64(&mut self, val: i64) {
        self.write_data(&val.to_ne_bytes());
    }

    /// Writes a 64-bit unsigned integer.
    pub fn write_u64(&mut self, val: u64) {
        self.write_data(&val.to_ne_bytes());
    }

    /// Reads a 32-bit signed integer.
    pub fn read_i32(&mut self) -> Option<i32> {
        let ptr = self.read_data(4)?;
        // SAFETY: We have at least 4 bytes available.
        Some(i32::from_ne_bytes(unsafe { *(ptr as *const [u8; 4]) }))
    }

    /// Reads a 32-bit unsigned integer.
    pub fn read_u32(&mut self) -> Option<u32> {
        let ptr = self.read_data(4)?;
        // SAFETY: We have at least 4 bytes available.
        Some(u32::from_ne_bytes(unsafe { *(ptr as *const [u8; 4]) }))
    }

    /// Reads a 64-bit signed integer.
    pub fn read_i64(&mut self) -> Option<i64> {
        let ptr = self.read_data(8)?;
        // SAFETY: We have at least 8 bytes available.
        Some(i64::from_ne_bytes(unsafe { *(ptr as *const [u8; 8]) }))
    }

    /// Reads a 64-bit unsigned integer.
    pub fn read_u64(&mut self) -> Option<u64> {
        let ptr = self.read_data(8)?;
        // SAFETY: We have at least 8 bytes available.
        Some(u64::from_ne_bytes(unsafe { *(ptr as *const [u8; 8]) }))
    }

    /// Writes a UTF-16 string (from ASCII).
    ///
    /// The string is converted to UTF-16 and null-terminated.
    pub fn write_string16(&mut self, s: &str) {
        let len = s.len();
        self.write_i32(len as i32);

        // Write UTF-16 characters (len+1 for null terminator)
        let char_count = len + 1;
        let byte_count = char_count * 2;

        if let Some(slice) = self.write_data_uninit(byte_count) {
            // Convert ASCII to UTF-16 (only works for ASCII strings)
            let mut i = 0;
            for byte in s.bytes() {
                if i + 1 < slice.len() {
                    slice[i] = byte;
                    slice[i + 1] = 0;
                    i += 2;
                }
            }
            // Null terminator
            if i + 1 < slice.len() {
                slice[i] = 0;
                slice[i + 1] = 0;
            }
        }
    }

    /// Writes an interface token (strict mode + interface name).
    pub fn write_interface_token(&mut self, interface: &str) {
        // Strict mode policy
        self.write_i32(0x100);
        self.write_string16(interface);
    }

    /// Reads a flattened object from the parcel.
    ///
    /// Returns the object data as a slice, or `None` if invalid.
    pub fn read_flattened_object(&mut self) -> Option<&[u8]> {
        let len = self.read_i32()?;
        let fd_count = self.read_i32()?;

        if fd_count != 0 {
            // FDs not supported
            return None;
        }

        if len < 0 {
            return None;
        }

        let ptr = self.read_data(len as usize)?;
        // SAFETY: We just read this data successfully.
        Some(unsafe { core::slice::from_raw_parts(ptr, len as usize) })
    }

    /// Writes a flattened object to the parcel.
    pub fn write_flattened_object(&mut self, data: &[u8]) -> Option<*mut u8> {
        self.write_i32(data.len() as i32); // len
        self.write_i32(0); // fd_count
        self.write_data(data)
    }
}

impl Default for Parcel {
    fn default() -> Self {
        Self::new()
    }
}
