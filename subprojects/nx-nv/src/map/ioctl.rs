//! The request payloads the memory-object device reads and writes.
//!
//! Every request is a fixed-layout struct the driver updates in place, and the
//! request code carries the payload's size. Deriving the code from the struct
//! keeps the two from drifting: adding a field changes the code automatically,
//! where a hand-written constant would keep naming the old size and the driver
//! would reject the request.

use nx_service_nv::{
    NV_IOC_READ,
    NV_IOC_WRITE,
};

/// The device's request type byte, shared by every request below.
const TYPE: u32 = 0x01;

/// Builds the request code for a payload the driver both reads and writes.
const fn request_code<T>(number: u32) -> u32 {
    let dir = NV_IOC_READ | NV_IOC_WRITE;
    let size = size_of::<T>() as u32;
    (dir << 30) | (size << 16) | (TYPE << 8) | number
}

/// Allocates a memory object of `size` bytes and returns its handle.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Create {
    /// In: the object's size in bytes.
    pub size: u32,
    /// Out: the handle naming the new object.
    pub handle: u32,
}

/// The request code for [`Create`].
pub const CREATE: u32 = request_code::<Create>(0x01);

/// Resolves a cross-process id to a handle in this process.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct FromId {
    /// In: the id to resolve.
    pub id: u32,
    /// Out: the handle naming the object in this process.
    pub handle: u32,
}

/// The request code for [`FromId`].
pub const FROM_ID: u32 = request_code::<FromId>(0x03);

/// Binds a CPU buffer to a memory object.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Alloc {
    /// In: the object to bind to.
    pub handle: u32,
    /// In: which heaps the object may be placed in; zero lets the driver choose.
    pub heapmask: u32,
    /// In: bit 0 requests a CPU-cacheable mapping.
    pub flags: u32,
    /// In: the alignment the object's pages are placed at.
    pub align: u32,
    /// In: the memory layout the GPU reads the object with.
    pub kind: u8,
    /// Padding to the 8-byte alignment `addr` needs.
    pub pad: [u8; 7],
    /// In: the CPU address of the buffer backing the object.
    pub addr: u64,
}

/// The request code for [`Alloc`].
pub const ALLOC: u32 = request_code::<Alloc>(0x04);

/// Drops this process's reference to a memory object.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Free {
    /// In: the object to release.
    pub handle: u32,
    /// Padding to the 8-byte alignment `refcount` needs.
    pub pad: u32,
    /// Out: references remaining after this one was dropped.
    pub refcount: u64,
    /// Out: the object's size in bytes.
    pub size: u32,
    /// Out: bit 0 is set when the object outlived this reference.
    pub flags: u32,
}

/// The request code for [`Free`].
pub const FREE: u32 = request_code::<Free>(0x05);

/// Reads one property of a memory object.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Param {
    /// In: the object to query.
    pub handle: u32,
    /// In: which property to read, one of [`param`].
    pub param: u32,
    /// Out: the property's value.
    pub result: u32,
}

/// The request code for [`Param`].
pub const PARAM: u32 = request_code::<Param>(0x09);

/// The properties [`Param`] can read.
pub mod param {
    /// The object's size in bytes.
    pub const SIZE: u32 = 1;
    /// The memory layout the GPU reads the object with.
    pub const KIND: u32 = 5;
}

/// Reads the cross-process id of a memory object.
///
/// The output field comes first here, unlike every other request on this
/// device.
#[derive(
    Debug,
    Default,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct GetId {
    /// Out: the id naming the object across processes.
    pub id: u32,
    /// In: the object to query.
    pub handle: u32,
}

/// The request code for [`GetId`].
pub const GET_ID: u32 = request_code::<GetId>(0x0E);

#[cfg(test)]
mod tests {
    use super::*;

    /// Splits a request code back into the four fields the driver reads.
    fn decode(code: u32) -> (u32, u32, u32, u32) {
        let dir = (code >> 30) & 0x3;
        let size = (code >> 16) & 0x3FFF;
        let kind = (code >> 8) & 0xFF;
        let number = code & 0xFF;
        (dir, size, kind, number)
    }

    #[test]
    fn create_request_code_carries_its_payload_size() {
        //* Given
        let expected_size = size_of::<Create>() as u32;

        //* When
        let (dir, size, kind, number) = decode(CREATE);

        //* Then
        assert_eq!(dir, 0x3, "the driver both reads and writes the payload");
        assert_eq!(size, expected_size, "the code must carry the payload size");
        assert_eq!(kind, TYPE, "every request on this device shares one type");
        assert_eq!(number, 0x01, "create is request one");
    }

    #[test]
    fn every_request_code_names_its_own_payload_size() {
        //* Given
        let codes = [
            (CREATE, size_of::<Create>()),
            (FROM_ID, size_of::<FromId>()),
            (ALLOC, size_of::<Alloc>()),
            (FREE, size_of::<Free>()),
            (PARAM, size_of::<Param>()),
            (GET_ID, size_of::<GetId>()),
        ];

        //* When / Then
        for (code, payload) in codes {
            let (_, size, _, _) = decode(code);
            assert_eq!(
                size as usize, payload,
                "code {code:#x} names the wrong size"
            );
        }
    }

    #[test]
    fn request_numbers_are_distinct() {
        //* Given
        let codes = [CREATE, FROM_ID, ALLOC, FREE, PARAM, GET_ID];

        //* When
        let numbers = codes.map(|code| decode(code).3);

        //* Then
        for (at, number) in numbers.iter().enumerate() {
            let duplicate = numbers[at + 1..].iter().any(|other| other == number);
            assert!(!duplicate, "request {number:#x} is used twice");
        }
    }

    #[test]
    fn alloc_places_the_address_at_an_eight_byte_boundary() {
        //* Given
        let payload = Alloc::default();

        //* When
        let base = &raw const payload;
        let addr = &raw const payload.addr;

        //* Then
        let offset = addr as usize - base as usize;
        assert_eq!(offset, 16, "the driver reads the address at offset 16");
        assert_eq!(size_of::<Alloc>(), 24, "a larger payload would be rejected");
    }

    #[test]
    fn get_id_puts_its_output_field_first() {
        //* Given
        let payload = GetId::default();

        //* When
        let base = &raw const payload;
        let handle = &raw const payload.handle;

        //* Then
        let offset = handle as usize - base as usize;
        assert_eq!(offset, 4, "the handle follows the id on this request");
    }
}
