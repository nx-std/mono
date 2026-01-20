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

/// NV driver error codes.
///
/// These map to libnx's LibnxNvidiaError_* constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum NvError {
    /// Operation not implemented.
    NotImplemented = 1,
    /// Operation not supported.
    NotSupported = 2,
    /// Service not initialized.
    NotInitialized = 3,
    /// Bad parameter provided.
    BadParameter = 4,
    /// Operation timed out.
    Timeout = 5,
    /// Insufficient memory available.
    InsufficientMemory = 6,
    /// Attribute is read-only.
    ReadOnlyAttribute = 7,
    /// Invalid state for operation.
    InvalidState = 8,
    /// Invalid memory address.
    InvalidAddress = 9,
    /// Invalid size specified.
    InvalidSize = 10,
    /// Bad value provided.
    BadValue = 11,
    /// Resource already allocated.
    AlreadyAllocated = 13,
    /// Resource is busy.
    Busy = 14,
    /// Resource error.
    ResourceError = 15,
    /// Count mismatch.
    CountMismatch = 16,
    /// Shared memory too small.
    SharedMemoryTooSmall = 0x1000,
    /// File operation failed.
    FileOperationFailed = 0x30003,
    /// Ioctl operation failed.
    IoctlFailed = 0x3000F,
    /// Unknown error.
    Unknown = -1,
}

impl NvError {
    /// Converts a raw NV error code to an NvError variant.
    pub fn from_raw(rc: i32) -> Self {
        match rc {
            0 => unreachable!("0 is success, not an error"),
            1 => Self::NotImplemented,
            2 => Self::NotSupported,
            3 => Self::NotInitialized,
            4 => Self::BadParameter,
            5 => Self::Timeout,
            6 => Self::InsufficientMemory,
            7 => Self::ReadOnlyAttribute,
            8 => Self::InvalidState,
            9 => Self::InvalidAddress,
            10 => Self::InvalidSize,
            11 => Self::BadValue,
            13 => Self::AlreadyAllocated,
            14 => Self::Busy,
            15 => Self::ResourceError,
            16 => Self::CountMismatch,
            0x1000 => Self::SharedMemoryTooSmall,
            0x30003 => Self::FileOperationFailed,
            0x3000F => Self::IoctlFailed,
            _ => Self::Unknown,
        }
    }

    /// Converts this NvError to a libnx-compatible result code.
    ///
    /// Uses Module_LibnxNvidia (346) as the module.
    pub fn to_result_code(self) -> u32 {
        const MODULE_LIBNX_NVIDIA: u32 = 346;

        // libnx error descriptor mapping
        let desc: u32 = match self {
            Self::NotImplemented => 1,
            Self::NotSupported => 2,
            Self::NotInitialized => 3,
            Self::BadParameter => 4,
            Self::Timeout => 5,
            Self::InsufficientMemory => 6,
            Self::ReadOnlyAttribute => 7,
            Self::InvalidState => 8,
            Self::InvalidAddress => 9,
            Self::InvalidSize => 10,
            Self::BadValue => 11,
            Self::AlreadyAllocated => 12,
            Self::Busy => 13,
            Self::ResourceError => 14,
            Self::CountMismatch => 15,
            Self::SharedMemoryTooSmall => 16,
            Self::FileOperationFailed => 17,
            Self::IoctlFailed => 18,
            Self::Unknown => 19,
        };

        // MAKERESULT(module, description) = ((module & 0x1FF) | ((description & 0x1FFF) << 9))
        (MODULE_LIBNX_NVIDIA & 0x1FF) | ((desc & 0x1FFF) << 9)
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
