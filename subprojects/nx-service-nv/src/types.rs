//! NV service data types.

/// NV service type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum NvServiceType {
    /// Auto-detect based on applet type.
    #[default]
    Auto = -1,
    /// Application service (nvdrv).
    Application = 0,
    /// Applet service (nvdrv:a).
    Applet = 1,
    /// System service (nvdrv:s).
    System = 2,
    /// Factory service (nvdrv:t).
    Factory = 3,
}

impl NvServiceType {
    /// Creates an NvServiceType from a raw i32 value.
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            -1 => Some(Self::Auto),
            0 => Some(Self::Application),
            1 => Some(Self::Applet),
            2 => Some(Self::System),
            3 => Some(Self::Factory),
            _ => None,
        }
    }
}

/// Error codes returned by NV Open command.
///
/// The Open command can return a limited set of error codes based on
/// whether the device exists and the service is ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenNvError {
    /// Device not supported or doesn't exist.
    #[error("device not supported")]
    NotSupported,
    /// Service not initialized.
    #[error("service not initialized")]
    NotInitialized,
    /// File operation failed (device couldn't be opened).
    #[error("file operation failed")]
    FileOperationFailed,
    /// Unknown or undocumented error code.
    #[error("unknown error code: {0:#x}")]
    Unknown(u32),
}

impl OpenNvError {
    /// Converts a raw NV error code to an `OpenNvError`.
    pub fn from_raw(code: u32) -> Self {
        match code {
            0x2 => Self::NotSupported,
            0x3 => Self::NotInitialized,
            0x30003 => Self::FileOperationFailed,
            other => Self::Unknown(other),
        }
    }

    /// Returns the raw NV error code.
    pub fn to_raw(self) -> u32 {
        match self {
            Self::NotSupported => 0x2,
            Self::NotInitialized => 0x3,
            Self::FileOperationFailed => 0x30003,
            Self::Unknown(code) => code,
        }
    }
}

/// Error codes returned by NV Close command.
///
/// The Close command can return errors related to service state and
/// whether the operation is implemented for the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CloseNvError {
    /// Operation not implemented for this device.
    #[error("not implemented")]
    NotImplemented,
    /// Service not initialized.
    #[error("service not initialized")]
    NotInitialized,
    /// Invalid state for operation.
    #[error("invalid state")]
    InvalidState,
    /// Unknown or undocumented error code.
    #[error("unknown error code: {0:#x}")]
    Unknown(u32),
}

impl CloseNvError {
    /// Converts a raw NV error code to a `CloseNvError`.
    pub fn from_raw(code: u32) -> Self {
        match code {
            0x1 => Self::NotImplemented,
            0x3 => Self::NotInitialized,
            0x8 => Self::InvalidState,
            other => Self::Unknown(other),
        }
    }

    /// Returns the raw NV error code.
    pub fn to_raw(self) -> u32 {
        match self {
            Self::NotImplemented => 0x1,
            Self::NotInitialized => 0x3,
            Self::InvalidState => 0x8,
            Self::Unknown(code) => code,
        }
    }
}

/// Error codes returned by NV QueryEvent command.
///
/// The QueryEvent command can fail due to service state, invalid parameters,
/// or if the event query is not implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QueryEventNvError {
    /// Operation not implemented for this device/event.
    #[error("not implemented")]
    NotImplemented,
    /// Service not initialized.
    #[error("service not initialized")]
    NotInitialized,
    /// Bad parameter (invalid event ID or fd).
    #[error("bad parameter")]
    BadParameter,
    /// Invalid state for operation.
    #[error("invalid state")]
    InvalidState,
    /// Unknown or undocumented error code.
    #[error("unknown error code: {0:#x}")]
    Unknown(u32),
}

impl QueryEventNvError {
    /// Converts a raw NV error code to a `QueryEventNvError`.
    pub fn from_raw(code: u32) -> Self {
        match code {
            0x1 => Self::NotImplemented,
            0x3 => Self::NotInitialized,
            0x4 => Self::BadParameter,
            0x8 => Self::InvalidState,
            other => Self::Unknown(other),
        }
    }

    /// Returns the raw NV error code.
    pub fn to_raw(self) -> u32 {
        match self {
            Self::NotImplemented => 0x1,
            Self::NotInitialized => 0x3,
            Self::BadParameter => 0x4,
            Self::InvalidState => 0x8,
            Self::Unknown(code) => code,
        }
    }
}

/// Error codes returned by NV Ioctl commands (Ioctl, Ioctl2, Ioctl3).
///
/// Ioctl commands delegate to device drivers which can return various
/// error codes. This enum covers the common cases with an `Unknown` variant
/// for device-specific or rare error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IoctlNvError {
    /// Operation not implemented.
    #[error("not implemented")]
    NotImplemented,
    /// Service not initialized.
    #[error("service not initialized")]
    NotInitialized,
    /// Bad parameter provided.
    #[error("bad parameter")]
    BadParameter,
    /// Operation timed out.
    #[error("timeout")]
    Timeout,
    /// Insufficient memory available.
    #[error("insufficient memory")]
    InsufficientMemory,
    /// Invalid state for operation.
    #[error("invalid state")]
    InvalidState,
    /// Bad value provided.
    #[error("bad value")]
    BadValue,
    /// Resource is busy.
    #[error("busy")]
    Busy,
    /// Unknown or device-specific error code.
    #[error("unknown error code: {0:#x}")]
    Unknown(u32),
}

impl IoctlNvError {
    /// Converts a raw NV error code to an `IoctlNvError`.
    pub fn from_raw(code: u32) -> Self {
        match code {
            0x1 => Self::NotImplemented,
            0x3 => Self::NotInitialized,
            0x4 => Self::BadParameter,
            0x5 => Self::Timeout,
            0x6 => Self::InsufficientMemory,
            0x8 => Self::InvalidState,
            0xB => Self::BadValue,
            0xE => Self::Busy,
            other => Self::Unknown(other),
        }
    }

    /// Returns the raw NV error code.
    pub fn to_raw(self) -> u32 {
        match self {
            Self::NotImplemented => 0x1,
            Self::NotInitialized => 0x3,
            Self::BadParameter => 0x4,
            Self::Timeout => 0x5,
            Self::InsufficientMemory => 0x6,
            Self::InvalidState => 0x8,
            Self::BadValue => 0xB,
            Self::Busy => 0xE,
            Self::Unknown(code) => code,
        }
    }
}

/// NV configuration options.
#[derive(Debug, Clone)]
pub struct NvConfig {
    /// Service type to connect to.
    pub service_type: NvServiceType,
    /// Transfer memory size for GPU operations.
    pub transfer_mem_size: usize,
}

impl Default for NvConfig {
    fn default() -> Self {
        Self {
            service_type: NvServiceType::Auto,
            transfer_mem_size: 0x80_0000, // 8 MB default
        }
    }
}

// Ioctl direction flags (matching linux ioctl convention)
/// No data transfer.
pub const NV_IOC_NONE: u32 = 0;
/// Read from driver.
pub const NV_IOC_READ: u32 = 2;
/// Write to driver.
pub const NV_IOC_WRITE: u32 = 1;

/// Extracts the direction from an ioctl request code.
#[inline]
pub const fn nv_ioc_dir(request: u32) -> u32 {
    (request >> 30) & 0x3
}

/// Extracts the size from an ioctl request code.
#[inline]
pub const fn nv_ioc_size(request: u32) -> usize {
    ((request >> 16) & 0x3FFF) as usize
}

/// Event IDs for NV services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NvEventId {
    /// GPU SM Exception BPT Interrupt Report.
    GpuSmExceptionBptIntReport = 1,
    /// GPU SM Exception BPT Pause Report.
    GpuSmExceptionBptPauseReport = 2,
    /// GPU Error Notifier.
    GpuErrorNotifier = 3,
}

/// Creates a control syncpoint event ID.
#[inline]
pub const fn nv_event_id_ctrl_syncpt(slot: u32, syncpt: u32) -> u32 {
    (1 << 28) | ((syncpt) << 16) | slot
}
