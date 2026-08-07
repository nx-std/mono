//! Wire-layout types for the IAudioDevice service.

use static_assertions::const_assert_eq;

/// Audio device name — a fixed-size 0x100-byte string buffer.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct AudioDeviceName {
    pub name: [u8; 0x100],
}

const_assert_eq!(size_of::<AudioDeviceName>(), 0x100);

impl AudioDeviceName {
    /// Returns the device name as a byte slice, trimmed at the first NUL.
    pub fn as_str_bytes(&self) -> &[u8] {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len());
        &self.name[..end]
    }
}
