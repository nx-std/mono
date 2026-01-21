//! VI service data types.

/// Display ID - identifies a display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DisplayId(u64);

impl DisplayId {
    /// Creates a new DisplayId from a raw value.
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw u64 value for FFI/IPC calls.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

impl From<DisplayId> for u64 {
    #[inline]
    fn from(id: DisplayId) -> Self {
        id.0
    }
}

/// Layer ID - identifies a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LayerId(u64);

impl LayerId {
    /// Creates a new LayerId from a raw value.
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw u64 value for FFI/IPC calls.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

impl From<LayerId> for u64 {
    #[inline]
    fn from(id: LayerId) -> Self {
        id.0
    }
}

/// Binder object ID - identifies a binder object (IGraphicBufferProducer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BinderObjectId(i32);

impl BinderObjectId {
    /// Creates a new BinderObjectId from a raw value.
    #[inline]
    pub const fn new(raw: i32) -> Self {
        Self(raw)
    }

    /// Returns the raw i32 value for FFI/IPC calls.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl From<BinderObjectId> for i32 {
    #[inline]
    fn from(id: BinderObjectId) -> Self {
        id.0
    }
}

/// Display name - 64-byte fixed string.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct DisplayName([u8; 0x40]);

impl DisplayName {
    /// Creates a new empty DisplayName.
    pub const fn new() -> Self {
        Self([0; 0x40])
    }

    /// Creates a DisplayName from a string slice.
    ///
    /// The string is truncated if longer than 63 bytes (leaving room for null terminator).
    pub fn from_ascii(s: &str) -> Self {
        let mut data = [0u8; 0x40];
        let bytes = s.as_bytes();
        let len = bytes.len().min(0x3F); // Leave room for null terminator
        data[..len].copy_from_slice(&bytes[..len]);
        Self(data)
    }

    /// Returns the display name as a byte slice.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 0x40] {
        &self.0
    }

    /// Returns the display name as a mutable byte slice.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; 0x40] {
        &mut self.0
    }

    /// Returns the display name as a string slice (up to first null byte).
    pub fn as_str(&self) -> &str {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(0x40);
        // SAFETY: Display names should be ASCII strings from the system.
        // If not valid UTF-8, we fall back to empty string.
        core::str::from_utf8(&self.0[..end]).unwrap_or("")
    }
}

impl Default for DisplayName {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for DisplayName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("DisplayName").field(&self.as_str()).finish()
    }
}

/// Default display name.
pub const DEFAULT_DISPLAY: DisplayName = {
    let mut data = [0u8; 0x40];
    data[0] = b'D';
    data[1] = b'e';
    data[2] = b'f';
    data[3] = b'a';
    data[4] = b'u';
    data[5] = b'l';
    data[6] = b't';
    DisplayName(data)
};

/// VI service type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(i32)]
pub enum ViServiceType {
    /// Auto-detect (try Manager, then System, then Application).
    #[default]
    Default = -1,
    /// Application service (vi:u).
    Application = 0,
    /// System service (vi:s).
    System = 1,
    /// Manager service (vi:m).
    Manager = 2,
}

impl ViServiceType {
    /// Creates a ViServiceType from a raw i32 value.
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            -1 => Some(Self::Default),
            0 => Some(Self::Application),
            1 => Some(Self::System),
            2 => Some(Self::Manager),
            _ => None,
        }
    }

    /// Returns the raw i32 value.
    #[inline]
    pub const fn as_raw(self) -> i32 {
        self as i32
    }
}

/// Layer flags for layer creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ViLayerFlags {
    /// Default layer flags.
    Default = 1,
}

/// Layer scaling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum ViScalingMode {
    /// No scaling.
    None = 0,
    /// Scale to fit layer dimensions.
    #[default]
    FitToLayer = 2,
    /// Scale while preserving aspect ratio.
    PreserveAspectRatio = 4,
}

/// Display power state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ViPowerState {
    /// Screen is off.
    Off = 0,
    /// Screen is on but not scanning content (3.0.0+).
    NotScanning = 1,
    /// Screen is on (3.0.0+).
    On = 2,
}

/// Layer stack selection for captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum ViLayerStack {
    /// Default layer stack, includes all layers.
    #[default]
    Default = 0,
    /// Includes only layers for the LCD.
    Lcd = 1,
    /// Includes only layers for user screenshots.
    Screenshot = 2,
    /// Includes only layers for recording videos.
    Recording = 3,
    /// Includes only layers for the last applet-transition frame.
    LastFrame = 4,
    /// Captures some arbitrary layer (normally only for AM).
    Arbitrary = 5,
    /// Captures layers for the current application (debugging tools).
    ApplicationForDebug = 6,
    /// Layer stack for the empty display.
    Null = 10,
}

/// RGBA4444 color format (16-bit).
pub type ViColorRgba4444 = u16;

/// RGBA8888 color format (32-bit).
pub type ViColorRgba8888 = u32;
