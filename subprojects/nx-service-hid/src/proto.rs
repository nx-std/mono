//! HID protocol constants and types.

use nx_sf::ServiceName;

/// Service name for HID.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hid");

/// HID service command IDs
pub mod cmds {
    // IHidServer command: CreateAppletResource
    pub const INITIALIZE_APPLET_RESOURCE: u32 = 0;

    // Touch screen
    pub const ACTIVATE_TOUCH_SCREEN: u32 = 11;

    // Mouse
    pub const ACTIVATE_MOUSE: u32 = 21;

    // Keyboard
    pub const ACTIVATE_KEYBOARD: u32 = 31;

    // Gesture
    pub const ACTIVATE_GESTURE: u32 = 91;

    // Npad
    pub const SET_SUPPORTED_NPAD_STYLE_SET: u32 = 100;
    pub const GET_SUPPORTED_NPAD_STYLE_SET: u32 = 101;
    pub const SET_SUPPORTED_NPAD_ID_TYPE: u32 = 102;
    pub const ACTIVATE_NPAD_WITH_REVISION: u32 = 109;
    pub const GET_NPAD_JOY_HOLD_TYPE: u32 = 121;
}

/// How the system expects a pair of Joy-Cons to be held.
///
/// libnx calls this `HidNpadJoyHoldType`. The server reports it as a `u64`, of
/// which only the two values below are defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NpadJoyHoldType {
    /// Held upright, the pair acting as one controller.
    Vertical = 0,
    /// Held sideways, each Joy-Con acting as its own controller.
    Horizontal = 1,
}

impl NpadJoyHoldType {
    /// Returns the hold type `raw` names, or [`None`] when it names none.
    #[inline]
    pub const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            0 => Some(Self::Vertical),
            1 => Some(Self::Horizontal),
            _ => None,
        }
    }

    /// Returns the raw value a wire field carrying this hold type holds.
    ///
    /// Narrower than the `u64` [`Self::from_raw`] takes: the server reports the
    /// hold type in a 64-bit reply, while every struct field carrying it is 32
    /// bits wide.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// IAppletResource command IDs
pub mod applet_resource_cmds {
    pub const GET_SHARED_MEMORY_HANDLE: u32 = 0;
}
