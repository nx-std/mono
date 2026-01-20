//! Basic input state types.

/// Analog stick state with X and Y coordinates.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalogStickState {
    pub x: i32,
    pub y: i32,
}

/// 3D vector for sensor data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Trait for input state types that can be read from LIFO buffers.
pub trait InputState: Sized {
    /// The atomic storage type for this state.
    type Storage;

    /// Extract the sampling number from the state for consistency checking.
    fn sampling_number(&self) -> u64;

    /// Load the state from atomic storage.
    ///
    /// # Safety
    ///
    /// Caller must ensure the storage pointer is valid and aligned.
    unsafe fn load_from_storage(storage: &Self::Storage) -> Self;
}
