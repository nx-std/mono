//! Wire-layout types for the BPC service.

/// Sleep button state as reported by [`GetSleepButtonState`](crate::proto::GET_SLEEP_BUTTON_STATE).
///
/// Available on HOS [2.0.0–13.2.1].
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepButtonState {
    Held = 0,
    Released = 1,
}

impl SleepButtonState {
    /// Converts a raw `u8` value to a [`SleepButtonState`], returning `None`
    /// for unrecognised values.
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Held),
            1 => Some(Self::Released),
            _ => None,
        }
    }
}
